use crate::outline::BezierAnchor;

/// Evaluate a cubic Bezier at parameter t ∈ [0, 1].
/// Returns (x, y) given 4 control points.
fn cubic_bezier(p0: [f32; 2], p1: [f32; 2], p2: [f32; 2], p3: [f32; 2], t: f32) -> [f32; 2] {
    let u = 1.0 - t;
    let uu = u * u;
    let uuu = uu * u;
    let tt = t * t;
    let ttt = tt * t;
    [
        uuu * p0[0] + 3.0 * uu * t * p1[0] + 3.0 * u * tt * p2[0] + ttt * p3[0],
        uuu * p0[1] + 3.0 * uu * t * p1[1] + 3.0 * u * tt * p2[1] + ttt * p3[1],
    ]
}

/// A 2-segment cubic Bezier curve for nod animation timing.
///
/// 3 control points forming an open path:
///   - Start (0, 0): fixed position, adjustable outgoing handle
///   - Middle: free position and handles
///   - End (1, 0): fixed position, adjustable incoming handle
///
/// X axis = normalized time [0, 1], Y axis = nod intensity [0, 1].
#[derive(Clone, Debug)]
pub struct NodCurve {
    pub anchors: [BezierAnchor; 3],
}

impl Default for NodCurve {
    fn default() -> Self {
        Self {
            anchors: [
                // Start: fixed at (0, 0)
                BezierAnchor {
                    position: [0.0, 0.0],
                    handle_in: [0.0, 0.0],
                    handle_out: [0.15, 0.0],
                },
                // Middle: default at (0.4, 1.0)
                BezierAnchor {
                    position: [0.4, 1.0],
                    handle_in: [-0.1, 0.0],
                    handle_out: [0.1, 0.0],
                },
                // End: fixed at (1, 0)
                BezierAnchor {
                    position: [1.0, 0.0],
                    handle_in: [-0.15, 0.0],
                    handle_out: [0.0, 0.0],
                },
            ],
        }
    }
}

impl NodCurve {
    /// Evaluate the curve at normalized time `global_t` ∈ [0, 1].
    ///
    /// Returns the Y value (nod intensity) using segment-based parametric evaluation.
    /// The middle anchor's X position divides time into two segments.
    pub fn evaluate(&self, global_t: f32) -> f32 {
        let t = global_t.clamp(0.0, 1.0);
        let mx = self.anchors[1].position[0].clamp(0.01, 0.99);

        if t <= mx {
            // Segment 1: Start → Middle
            let local_t = t / mx;
            let p0 = self.anchors[0].position;
            let p1 = [
                p0[0] + self.anchors[0].handle_out[0],
                p0[1] + self.anchors[0].handle_out[1],
            ];
            let p3 = self.anchors[1].position;
            let p2 = [
                p3[0] + self.anchors[1].handle_in[0],
                p3[1] + self.anchors[1].handle_in[1],
            ];
            cubic_bezier(p0, p1, p2, p3, local_t)[1]
        } else {
            // Segment 2: Middle → End
            let local_t = (t - mx) / (1.0 - mx);
            let p0 = self.anchors[1].position;
            let p1 = [
                p0[0] + self.anchors[1].handle_out[0],
                p0[1] + self.anchors[1].handle_out[1],
            ];
            let p3 = self.anchors[2].position;
            let p2 = [
                p3[0] + self.anchors[2].handle_in[0],
                p3[1] + self.anchors[2].handle_in[1],
            ];
            cubic_bezier(p0, p1, p2, p3, local_t)[1]
        }
    }
}

/// Output of a single nod animation frame.
pub struct NodOutput {
    /// Face tilt angle in radians (>0 = forward nod).
    pub nod_pitch: f32,
    /// Vertical sink offset (screen-space, >0 = downward).
    pub nod_sink: f32,
    /// Eyelid close value (overrides normal eyelid state during nod).
    pub eyelid_close: f32,
}

struct NodEvent {
    start_time: f32,
    /// Eyelid close value captured at the moment the nod was triggered.
    initial_eyelid_close: f32,
}

/// Nod animation state machine.
///
/// Plays a single nod when triggered, modulating `look_y` and `eyelid_close`
/// according to a user-defined bezier timing curve.
pub struct NodAnimation {
    /// The timing/shape curve.
    pub curve: NodCurve,
    /// Maximum nod angle in radians (scales the curve output). 0.5 ≈ 28.6°.
    pub amount: f32,
    /// Maximum vertical sink depth in screen-space units.
    pub sink_depth: f32,
    /// Total duration of the nod in seconds.
    pub duration: f32,
    /// Eye closeness at the peak of the nod (middle point). 0.0 = fully open, 1.0 = fully closed.
    pub mid_closeness: f32,
    /// Eye openness at the end of the nod (state B). 0.0 = fully open, 1.0 = fully closed.
    pub end_openness: f32,
    /// Rotation pivot Y position in screen space (-1.0 = bottom of screen).
    pub pivot_y: f32,
    /// Active event state. None when idle.
    active_event: Option<NodEvent>,
}

impl Default for NodAnimation {
    fn default() -> Self {
        Self {
            curve: NodCurve::default(),
            amount: 0.5,
            sink_depth: 0.0,
            duration: 0.5,
            mid_closeness: 1.0,
            end_openness: 0.0,
            pivot_y: -1.0,
            active_event: None,
        }
    }
}

impl NodAnimation {
    /// Start a nod at the given time, capturing the current eyelid state.
    pub fn trigger(&mut self, time: f32, current_eyelid_close: f32) {
        self.active_event = Some(NodEvent {
            start_time: time,
            initial_eyelid_close: current_eyelid_close,
        });
    }

    /// Whether a nod is currently in progress.
    pub fn is_active(&self) -> bool {
        self.active_event.is_some()
    }

    /// Evaluate the nod at the current time.
    ///
    /// Returns `Some(NodOutput)` while the nod is active, `None` when idle.
    /// Automatically clears the event when the nod finishes.
    pub fn evaluate(&mut self, time: f32) -> Option<NodOutput> {
        let event = self.active_event.as_ref()?;

        let elapsed = time - event.start_time;
        if elapsed < 0.0 {
            return None;
        }

        let global_t = elapsed / self.duration;
        if global_t >= 1.0 {
            self.active_event = None;
            return None;
        }

        let curve_y = self.curve.evaluate(global_t).max(0.0);

        // Face tilt angle: amount is max angle in radians
        let nod_pitch = self.amount * curve_y;

        // Vertical sink: same curve timing, scaled by sink_depth
        let nod_sink = self.sink_depth * curve_y;

        // Eyelid close: linear interpolation independent of the bezier curve.
        // Segment 1 (0→mx): initial → mid_closeness
        // Segment 2 (mx→1): mid_closeness → end_openness
        let mx = self.curve.anchors[1].position[0].clamp(0.01, 0.99);
        let eyelid_close = if global_t <= mx {
            let local_t = global_t / mx;
            let initial = event.initial_eyelid_close;
            initial + (self.mid_closeness - initial) * local_t
        } else {
            let local_t = (global_t - mx) / (1.0 - mx);
            self.mid_closeness + (self.end_openness - self.mid_closeness) * local_t
        };

        Some(NodOutput {
            nod_pitch,
            nod_sink,
            eyelid_close: eyelid_close.clamp(0.0, 1.0),
        })
    }
}
