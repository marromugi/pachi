use serde::{Deserialize, Serialize};

use crate::animation::{apply_easing, Easing};
use crate::config::{
    BezierAnchorConfig, BezierOutlineConfig, EyeShapeConfig, EyeSideConfig, EyebrowOutlineConfig,
    EyebrowShapeConfig, EyelashShapeConfig,
};

// ============================================================
// Timeline easing (serde-compatible wrapper for animation::Easing)
// ============================================================

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum TimelineEasing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
}

impl TimelineEasing {
    pub fn to_easing(self) -> Easing {
        match self {
            Self::Linear => Easing::Linear,
            Self::EaseIn => Easing::EaseIn,
            Self::EaseOut => Easing::EaseOut,
            Self::EaseInOut => Easing::EaseInOut,
        }
    }

    pub const ALL: [Self; 4] = [Self::Linear, Self::EaseIn, Self::EaseOut, Self::EaseInOut];

    pub fn label(self) -> &'static str {
        match self {
            Self::Linear => "Linear",
            Self::EaseIn => "Ease In",
            Self::EaseOut => "Ease Out",
            Self::EaseInOut => "Ease In/Out",
        }
    }
}

impl Default for TimelineEasing {
    fn default() -> Self {
        Self::EaseInOut
    }
}

// ============================================================
// Global config subset (interpolatable fields only)
// ============================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimelineGlobalConfig {
    pub bg_color: [f32; 3],
    pub eye_separation: f32,
    pub max_angle: f32,
    pub eye_angle: f32,
    pub focus_distance: f32,
}

// ============================================================
// Keyframe and Timeline
// ============================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimelineKeyframe {
    pub label: String,
    /// Time in seconds from timeline start when this keyframe is fully reached.
    pub fire_time: f32,
    /// Duration of transition FROM the previous keyframe TO this one.
    pub transition_duration: f32,
    pub easing: TimelineEasing,
    pub left: EyeSideConfig,
    pub right: EyeSideConfig,
    pub global: TimelineGlobalConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Timeline {
    pub keyframes: Vec<TimelineKeyframe>,
}

impl Timeline {
    pub fn new() -> Self {
        Self {
            keyframes: Vec::new(),
        }
    }

    pub fn total_duration(&self) -> f32 {
        self.keyframes.last().map(|kf| kf.fire_time).unwrap_or(0.0)
    }

    pub fn sort(&mut self) {
        self.keyframes
            .sort_by(|a, b| a.fire_time.partial_cmp(&b.fire_time).unwrap());
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

// ============================================================
// Interpolation: primitives
// ============================================================

fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn lerp_f32_2(a: [f32; 2], b: [f32; 2], t: f32) -> [f32; 2] {
    [lerp_f32(a[0], b[0], t), lerp_f32(a[1], b[1], t)]
}

fn lerp_f32_3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        lerp_f32(a[0], b[0], t),
        lerp_f32(a[1], b[1], t),
        lerp_f32(a[2], b[2], t),
    ]
}

fn snap_bool_2(a: [bool; 2], b: [bool; 2], t: f32) -> [bool; 2] {
    if t >= 1.0 {
        b
    } else {
        a
    }
}

// ============================================================
// Interpolation: compound types
// ============================================================

fn lerp_anchor(a: &BezierAnchorConfig, b: &BezierAnchorConfig, t: f32) -> BezierAnchorConfig {
    BezierAnchorConfig {
        position: lerp_f32_2(a.position, b.position, t),
        handle_in: lerp_f32_2(a.handle_in, b.handle_in, t),
        handle_out: lerp_f32_2(a.handle_out, b.handle_out, t),
    }
}

fn lerp_outline(a: &BezierOutlineConfig, b: &BezierOutlineConfig, t: f32) -> BezierOutlineConfig {
    BezierOutlineConfig {
        anchors: core::array::from_fn(|i| lerp_anchor(&a.anchors[i], &b.anchors[i], t)),
    }
}

fn lerp_eyebrow_outline(
    a: &EyebrowOutlineConfig,
    b: &EyebrowOutlineConfig,
    t: f32,
) -> EyebrowOutlineConfig {
    EyebrowOutlineConfig {
        anchors: core::array::from_fn(|i| lerp_anchor(&a.anchors[i], &b.anchors[i], t)),
    }
}

fn lerp_eye_shape(a: &EyeShapeConfig, b: &EyeShapeConfig, t: f32) -> EyeShapeConfig {
    EyeShapeConfig {
        open: lerp_outline(&a.open, &b.open, t),
        closed: lerp_outline(&a.closed, &b.closed, t),
        close_arch: lerp_f32(a.close_arch, b.close_arch, t),
    }
}

fn lerp_eyebrow_shape(
    a: &EyebrowShapeConfig,
    b: &EyebrowShapeConfig,
    t: f32,
) -> EyebrowShapeConfig {
    EyebrowShapeConfig {
        outline: lerp_eyebrow_outline(&a.outline, &b.outline, t),
        thickness: lerp_f32_3(a.thickness, b.thickness, t),
        tip_round: snap_bool_2(a.tip_round, b.tip_round, t),
        base_y: lerp_f32(a.base_y, b.base_y, t),
        follow: lerp_f32(a.follow, b.follow, t),
        color: lerp_f32_3(a.color, b.color, t),
    }
}

fn lerp_eyelash_shape(
    a: &EyelashShapeConfig,
    b: &EyelashShapeConfig,
    t: f32,
) -> EyelashShapeConfig {
    EyelashShapeConfig {
        color: lerp_f32_3(a.color, b.color, t),
        thickness: lerp_f32(a.thickness, b.thickness, t),
    }
}

// ============================================================
// Interpolation: full config
// ============================================================

pub fn lerp_eye_side(a: &EyeSideConfig, b: &EyeSideConfig, t: f32) -> EyeSideConfig {
    EyeSideConfig {
        sclera_color: lerp_f32_3(a.sclera_color, b.sclera_color, t),
        iris_color: lerp_f32_3(a.iris_color, b.iris_color, t),
        pupil_color: lerp_f32_3(a.pupil_color, b.pupil_color, t),
        eyelid_close: lerp_f32(a.eyelid_close, b.eyelid_close, t),
        iris_radius: lerp_f32(a.iris_radius, b.iris_radius, t),
        iris_follow: lerp_f32(a.iris_follow, b.iris_follow, t),
        iris_offset_y: lerp_f32(a.iris_offset_y, b.iris_offset_y, t),
        pupil_radius: lerp_f32(a.pupil_radius, b.pupil_radius, t),
        highlight_offset: lerp_f32_2(a.highlight_offset, b.highlight_offset, t),
        highlight_radius: lerp_f32(a.highlight_radius, b.highlight_radius, t),
        highlight_intensity: lerp_f32(a.highlight_intensity, b.highlight_intensity, t),
        highlight_blur: lerp_f32(a.highlight_blur, b.highlight_blur, t),
        look_x: lerp_f32(a.look_x, b.look_x, t),
        look_y: lerp_f32(a.look_y, b.look_y, t),
        eye_shape: lerp_eye_shape(&a.eye_shape, &b.eye_shape, t),
        eyebrow_shape: lerp_eyebrow_shape(&a.eyebrow_shape, &b.eyebrow_shape, t),
        eyelash_shape: lerp_eyelash_shape(&a.eyelash_shape, &b.eyelash_shape, t),
        iris_shape: lerp_outline(&a.iris_shape, &b.iris_shape, t),
        pupil_shape: lerp_outline(&a.pupil_shape, &b.pupil_shape, t),
    }
}

pub fn lerp_timeline_global(
    a: &TimelineGlobalConfig,
    b: &TimelineGlobalConfig,
    t: f32,
) -> TimelineGlobalConfig {
    TimelineGlobalConfig {
        bg_color: lerp_f32_3(a.bg_color, b.bg_color, t),
        eye_separation: lerp_f32(a.eye_separation, b.eye_separation, t),
        max_angle: lerp_f32(a.max_angle, b.max_angle, t),
        eye_angle: lerp_f32(a.eye_angle, b.eye_angle, t),
        focus_distance: lerp_f32(a.focus_distance, b.focus_distance, t),
    }
}

// ============================================================
// Timeline output frame
// ============================================================

pub struct TimelineFrame {
    pub left: EyeSideConfig,
    pub right: EyeSideConfig,
    pub global: TimelineGlobalConfig,
}

// ============================================================
// Playback state machine
// ============================================================

pub struct TimelinePlayer {
    pub timeline: Timeline,
    pub playing: bool,
    pub looping: bool,
    elapsed: f32,
    play_start_wall: f32,
    elapsed_at_pause: f32,
    pub selected_keyframe: Option<usize>,
}

impl TimelinePlayer {
    pub fn new() -> Self {
        Self {
            timeline: Timeline::new(),
            playing: false,
            looping: false,
            elapsed: 0.0,
            play_start_wall: 0.0,
            elapsed_at_pause: 0.0,
            selected_keyframe: None,
        }
    }

    pub fn play(&mut self, wall_time: f32) {
        if !self.playing {
            self.play_start_wall = wall_time;
            self.playing = true;
        }
    }

    pub fn stop(&mut self) {
        self.playing = false;
        self.elapsed = 0.0;
        self.elapsed_at_pause = 0.0;
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn current_time(&self) -> f32 {
        self.elapsed
    }

    pub fn evaluate(&mut self, wall_time: f32) -> Option<TimelineFrame> {
        if !self.playing || self.timeline.keyframes.is_empty() {
            return None;
        }

        self.elapsed = self.elapsed_at_pause + (wall_time - self.play_start_wall);

        let total = self.timeline.total_duration();
        if total <= 0.0 {
            let kf = &self.timeline.keyframes[0];
            return Some(TimelineFrame {
                left: kf.left.clone(),
                right: kf.right.clone(),
                global: kf.global.clone(),
            });
        }

        if self.elapsed > total {
            if self.looping {
                self.elapsed %= total;
                self.play_start_wall = wall_time;
                self.elapsed_at_pause = self.elapsed;
            } else {
                self.playing = false;
                let kf = self.timeline.keyframes.last().unwrap();
                return Some(TimelineFrame {
                    left: kf.left.clone(),
                    right: kf.right.clone(),
                    global: kf.global.clone(),
                });
            }
        }

        self.interpolate_at(self.elapsed)
    }

    fn interpolate_at(&self, t: f32) -> Option<TimelineFrame> {
        let kfs = &self.timeline.keyframes;
        if kfs.is_empty() {
            return None;
        }

        // Before or at first keyframe: hold first keyframe
        if t <= kfs[0].fire_time {
            return Some(TimelineFrame {
                left: kfs[0].left.clone(),
                right: kfs[0].right.clone(),
                global: kfs[0].global.clone(),
            });
        }

        // Find the segment: kfs[i-1] ... kfs[i]
        for i in 1..kfs.len() {
            if t <= kfs[i].fire_time {
                let prev = &kfs[i - 1];
                let curr = &kfs[i];

                let transition_start =
                    (curr.fire_time - curr.transition_duration).max(prev.fire_time);

                if t < transition_start {
                    // Hold at previous keyframe (gap before transition)
                    return Some(TimelineFrame {
                        left: prev.left.clone(),
                        right: prev.right.clone(),
                        global: prev.global.clone(),
                    });
                }

                // Within transition
                let transition_elapsed = t - transition_start;
                let transition_total = curr.fire_time - transition_start;
                let raw_t = if transition_total > 0.0 {
                    (transition_elapsed / transition_total).clamp(0.0, 1.0)
                } else {
                    1.0
                };

                let eased_t = apply_easing(raw_t, curr.easing.to_easing());

                return Some(TimelineFrame {
                    left: lerp_eye_side(&prev.left, &curr.left, eased_t),
                    right: lerp_eye_side(&prev.right, &curr.right, eased_t),
                    global: lerp_timeline_global(&prev.global, &curr.global, eased_t),
                });
            }
        }

        // Past last keyframe: hold at last
        let last = kfs.last().unwrap();
        Some(TimelineFrame {
            left: last.left.clone(),
            right: last.right.clone(),
            global: last.global.clone(),
        })
    }
}
