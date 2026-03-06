use crate::animation::{apply_easing, Easing, Xorshift32};

struct MicrosaccadeEvent {
    start_time: f32,
    /// Starting offset (before this saccade).
    from_x: f32,
    from_y: f32,
    /// Target offset (after this saccade).
    to_x: f32,
    to_y: f32,
    /// Transition duration in seconds.
    duration: f32,
}

/// Microsaccade animation: tiny, quick involuntary iris/pupil shifts.
///
/// When triggered, both eyes shift in the same random direction
/// (biased toward center). The iris stays at the new position
/// until the next trigger.
pub struct MicrosaccadeAnimation {
    rng: Xorshift32,
    active_event: Option<MicrosaccadeEvent>,
    /// Current resting offset (persists after animation completes).
    pub offset_x: f32,
    pub offset_y: f32,
    /// How strongly the direction is biased toward center (0.0 = none, 1.0 = always toward center).
    center_bias: f32,
}

impl MicrosaccadeAnimation {
    pub fn new(seed: u32) -> Self {
        Self {
            rng: Xorshift32::new(seed),
            active_event: None,
            offset_x: 0.0,
            offset_y: 0.0,
            center_bias: 0.6,
        }
    }

    /// Trigger a microsaccade at the given time.
    ///
    /// `current_look_x` and `current_look_y` are the current gaze direction,
    /// used to bias the saccade toward the center.
    pub fn trigger(&mut self, time: f32, current_look_x: f32, current_look_y: f32) {
        let from_x = self.offset_x;
        let from_y = self.offset_y;

        // Random direction (angle in radians)
        let angle = self.rng.range(0.0, std::f32::consts::TAU);
        let random_dx = angle.cos();
        let random_dy = angle.sin();

        // Center direction: bias toward (0, 0) in iris-offset space
        let cur_x = current_look_x + from_x;
        let cur_y = current_look_y + from_y;
        let center_len = (cur_x * cur_x + cur_y * cur_y).sqrt();
        let (center_dx, center_dy) = if center_len > 0.001 {
            (-cur_x / center_len, -cur_y / center_len)
        } else {
            (random_dx, random_dy)
        };

        // Mix random direction with center-biased direction
        let bias = self.center_bias;
        let dx = random_dx * (1.0 - bias) + center_dx * bias;
        let dy = random_dy * (1.0 - bias) + center_dy * bias;

        // Normalize
        let len = (dx * dx + dy * dy).sqrt();
        let (dx, dy) = if len > 0.001 {
            (dx / len, dy / len)
        } else {
            (1.0, 0.0)
        };

        // Very small amplitude in screen space (iris_follow ≈ 0.14, so this is tiny)
        let amplitude = self.rng.range(0.024, 0.028);
        let duration = self.rng.range(0.03, 0.06);

        let to_x = dx * amplitude;
        let to_y = dy * amplitude;

        self.active_event = Some(MicrosaccadeEvent {
            start_time: time,
            from_x,
            from_y,
            to_x,
            to_y,
            duration,
        });
    }

    /// Evaluate the current iris offset.
    ///
    /// Returns `(offset_x, offset_y)` — always valid (persists after animation).
    pub fn evaluate(&mut self, time: f32) -> (f32, f32) {
        if let Some(ref event) = self.active_event {
            let elapsed = time - event.start_time;
            if elapsed < 0.0 {
                return (self.offset_x, self.offset_y);
            }

            let t = (elapsed / event.duration).min(1.0);
            let eased = apply_easing(t, Easing::EaseOut);

            let ox = event.from_x + (event.to_x - event.from_x) * eased;
            let oy = event.from_y + (event.to_y - event.from_y) * eased;

            if t >= 1.0 {
                // Animation done — commit final offset
                self.offset_x = event.to_x;
                self.offset_y = event.to_y;
                self.active_event = None;
            }

            (ox, oy)
        } else {
            (self.offset_x, self.offset_y)
        }
    }
}
