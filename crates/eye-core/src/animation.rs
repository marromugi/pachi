#[derive(Clone, Copy)]
enum Easing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
}

#[derive(Clone, Copy)]
struct Keyframe {
    time: f32,
    value: f32,
    easing: Easing,
}

pub struct BlinkAnimation {
    keyframes: Vec<Keyframe>,
    period: f32,
}

impl BlinkAnimation {
    /// Construct the default sample blink animation.
    ///
    /// Loop period: 5.0 seconds.
    /// Contains: 2.1s rest, double-blink, 1s rest, lazy half-squint, return to rest.
    pub fn sample() -> Self {
        let keyframes = vec![
            Keyframe { time: 0.00, value: 0.20, easing: Easing::Linear },
            Keyframe { time: 1.00, value: 0.20, easing: Easing::Linear },
            Keyframe { time: 1.12, value: 1.00, easing: Easing::EaseIn },
            Keyframe { time: 1.22, value: 0.45, easing: Easing::EaseOut },
            Keyframe { time: 1.32, value: 1.00, easing: Easing::EaseIn },
            Keyframe { time: 1.57, value: 0.20, easing: Easing::EaseInOut },
            Keyframe { time: 2.10, value: 0.20, easing: Easing::Linear },
            Keyframe { time: 2.20, value: 0.50, easing: Easing::EaseIn },
            Keyframe { time: 2.40, value: 0.50, easing: Easing::Linear },
            Keyframe { time: 2.55, value: 0.20, easing: Easing::EaseOut },
            Keyframe { time: 3.00, value: 0.20, easing: Easing::Linear },
        ];
        Self { keyframes, period: 3.0 }
    }

    /// Evaluate `eyelid_close` at absolute application time `t` (seconds).
    /// The animation loops with period `self.period`.
    pub fn evaluate(&self, t: f32) -> f32 {
        let loop_t = t.rem_euclid(self.period);

        let next_idx = self.keyframes
            .iter()
            .position(|kf| kf.time > loop_t)
            .unwrap_or(self.keyframes.len() - 1);

        if next_idx == 0 {
            return self.keyframes[0].value;
        }

        let prev = &self.keyframes[next_idx - 1];
        let next = &self.keyframes[next_idx];

        let segment_duration = next.time - prev.time;
        if segment_duration < 1e-7 {
            return next.value;
        }

        let raw_t = (loop_t - prev.time) / segment_duration;
        let eased = apply_easing(raw_t, next.easing);

        prev.value + (next.value - prev.value) * eased
    }
}

fn apply_easing(t: f32, easing: Easing) -> f32 {
    match easing {
        Easing::Linear => t,
        Easing::EaseIn => t * t,
        Easing::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
        Easing::EaseInOut => {
            if t < 0.5 {
                2.0 * t * t
            } else {
                1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
            }
        }
    }
}
