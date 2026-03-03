"""
Face tracker that detects eye positions via webcam and sends gaze parameters
to the eye renderer over WebSocket.

Usage:
    pip install -r requirements.txt
    python face_tracker.py

The eye renderer (cargo run --example desktop) must be running first.
"""

import asyncio
import json
import os
import sys
import time

import cv2
import mediapipe as mp
import websockets

# MediaPipe Tasks API (replaces legacy mp.solutions)
BaseOptions = mp.tasks.BaseOptions
FaceLandmarker = mp.tasks.vision.FaceLandmarker
FaceLandmarkerOptions = mp.tasks.vision.FaceLandmarkerOptions
VisionRunningMode = mp.tasks.vision.RunningMode

MODEL_PATH = os.path.join(os.path.dirname(__file__), "face_landmarker.task")

# Iris center landmark indices (478-point model)
LEFT_IRIS_CENTER = 468
RIGHT_IRIS_CENTER = 473

WS_URI = "ws://127.0.0.1:8765"
TARGET_FPS = 30


def ema(prev: float, current: float, alpha: float) -> float:
    """Exponential moving average."""
    return prev + alpha * (current - prev)


async def track():
    if not os.path.exists(MODEL_PATH):
        print(f"Error: Model file not found at {MODEL_PATH}", file=sys.stderr)
        print("Download it with:")
        print(
            '  curl -L -o face_landmarker.task "https://storage.googleapis.com/mediapipe-models/face_landmarker/face_landmarker/float16/latest/face_landmarker.task"'
        )
        sys.exit(1)

    options = FaceLandmarkerOptions(
        base_options=BaseOptions(model_asset_path=MODEL_PATH),
        running_mode=VisionRunningMode.VIDEO,
        num_faces=1,
    )
    landmarker = FaceLandmarker.create_from_options(options)

    cap = cv2.VideoCapture(0)
    if not cap.isOpened():
        print("Error: Could not open camera", file=sys.stderr)
        sys.exit(1)

    print(f"Connecting to {WS_URI} ...")
    try:
        ws = await websockets.connect(WS_URI)
    except Exception as e:
        print(f"Error: Could not connect to WebSocket server: {e}", file=sys.stderr)
        print("Make sure the eye renderer is running (cargo run --example desktop)")
        cap.release()
        sys.exit(1)

    print("Connected! Tracking face...")

    # EMA state
    prev_x = 0.0
    prev_y = 0.0
    prev_depth = 2.0
    alpha_xy = 0.3
    alpha_depth = 0.15
    frame_timestamp_ms = 0
    frame_count = 0
    fps_timer = time.monotonic()

    frame_interval = 1.0 / TARGET_FPS

    try:
        while cap.isOpened():
            loop_start = time.monotonic()

            ret, frame = cap.read()
            if not ret:
                break

            rgb = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
            mp_image = mp.Image(image_format=mp.ImageFormat.SRGB, data=rgb)
            frame_timestamp_ms += int(1000 / TARGET_FPS)

            result = landmarker.detect_for_video(mp_image, frame_timestamp_ms)

            # FPS counter
            frame_count += 1
            now = time.monotonic()
            elapsed = now - fps_timer
            if elapsed >= 1.0:
                fps = frame_count / elapsed
                face = "face" if result.face_landmarks else "no face"
                print(f"\r{fps:.1f} fps ({face})", end="", flush=True)
                frame_count = 0
                fps_timer = now

            if result.face_landmarks:
                landmarks = result.face_landmarks[0]
                h, w = frame.shape[:2]

                le = landmarks[LEFT_IRIS_CENTER]
                re = landmarks[RIGHT_IRIS_CENTER]

                # Face center in normalized coords [0, 1]
                cx = (le.x + re.x) / 2.0
                cy = (le.y + re.y) / 2.0

                # Map to look direction [-1, 1] (mirrored for camera)
                raw_x = -((cx - 0.5) * 2.0)
                raw_y = -((cy - 0.5) * 2.0)

                # Inter-pupillary distance in pixels for depth estimation
                dx = (re.x - le.x) * w
                dy = (re.y - le.y) * h
                eye_dist_px = (dx**2 + dy**2) ** 0.5
                raw_depth = max(0.5, min(20.0, 80.0 / max(eye_dist_px, 1.0)))

                # Smooth
                prev_x = ema(prev_x, raw_x, alpha_xy)
                prev_y = ema(prev_y, raw_y, alpha_xy)
                prev_depth = ema(prev_depth, raw_depth, alpha_depth)

                msg = json.dumps(
                    {
                        "look_x": round(prev_x, 4),
                        "look_y": round(prev_y, 4),
                        "focus_distance": round(prev_depth, 4),
                    }
                )
                await ws.send(msg)

            # Sleep only the remaining time to hit target FPS
            spent = time.monotonic() - loop_start
            remaining = frame_interval - spent
            if remaining > 0:
                await asyncio.sleep(remaining)

    except websockets.exceptions.ConnectionClosed:
        print("WebSocket connection closed")
    except KeyboardInterrupt:
        print("\nStopped")
    finally:
        cap.release()
        landmarker.close()
        await ws.close()


if __name__ == "__main__":
    asyncio.run(track())
