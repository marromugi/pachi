#[derive(Clone, Copy)]
#[allow(dead_code)]
enum Easing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
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

/// Lightweight xorshift32 PRNG (no external crate needed).
struct Xorshift32 {
    state: u32,
}

impl Xorshift32 {
    fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 1 } else { seed },
        }
    }

    /// Returns a pseudo-random u32.
    fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    /// Returns a pseudo-random f32 in [0, 1).
    fn next_f32(&mut self) -> f32 {
        (self.next_u32() & 0x00FF_FFFF) as f32 / 16_777_216.0
    }

    /// Returns a pseudo-random f32 in [lo, hi).
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.next_f32() * (hi - lo)
    }
}

/// A single blink event with asymmetric timing.
struct BlinkEvent {
    start_time: f32,
    close_duration: f32,
    open_duration: f32,
}

impl BlinkEvent {
    /// Total duration of this blink.
    fn total_duration(&self) -> f32 {
        self.close_duration + self.open_duration
    }

    fn end_time(&self) -> f32 {
        self.start_time + self.total_duration()
    }
}

/// Natural idle blink animation based on Disney Research findings.
///
/// Generates randomised blink events with asymmetric close/open timing
/// (close ≈ 30%, open ≈ 70%) and variable inter-blink intervals (4–10 s).
pub struct BlinkAnimation {
    rest_value: f32,
    rng: Xorshift32,
    current_blink: Option<BlinkEvent>,
    next_blink_time: f32,
    last_t: f32,
}

impl BlinkAnimation {
    /// Create a new blink animation with the given RNG seed and rest eyelid value.
    pub fn new(seed: u32, rest_value: f32) -> Self {
        let mut rng = Xorshift32::new(seed);
        let next_blink_time = rng.range(1.0, 3.0); // first blink comes a bit sooner
        Self {
            rest_value,
            rng,
            current_blink: None,
            next_blink_time,
            last_t: 0.0,
        }
    }

    /// Backward-compatible constructor (equivalent to `new(42, 0.20)`).
    pub fn sample() -> Self {
        Self::new(42, 0.20)
    }

    /// Trigger an immediate blink at time `t`.
    pub fn trigger(&mut self, t: f32) {
        self.current_blink = Some(self.generate_blink(t));
        let total = self.current_blink.as_ref().unwrap().total_duration();
        self.next_blink_time = t + total + self.rng.range(4.0, 10.0);
    }

    /// Advance internal state and return the current `eyelid_close` value.
    ///
    /// Must be called with monotonically increasing `t` (seconds since start).
    pub fn evaluate(&mut self, t: f32) -> f32 {
        self.advance(t);
        self.compute_value(t)
    }

    /// Return the `eyelid_close` value at time `t` **without** mutating state.
    ///
    /// Useful for computing velocity from a past sample without disturbing the
    /// scheduling timeline.
    pub fn peek_value(&self, t: f32) -> f32 {
        self.compute_value(t)
    }

    /// Internal: update scheduling (generate new blinks as needed).
    fn advance(&mut self, t: f32) {
        // Detect large time jumps (e.g. toggle off→on): reset scheduling.
        if t < self.last_t - 0.5 {
            self.current_blink = None;
            self.next_blink_time = t + self.rng.range(1.0, 3.0);
        }
        self.last_t = t;

        // If there is a finished blink, clear it.
        if let Some(ref blink) = self.current_blink {
            if t >= blink.end_time() {
                self.current_blink = None;
            }
        }

        // If no blink is active and it's time for the next one, start it.
        if self.current_blink.is_none() && t >= self.next_blink_time {
            self.current_blink = Some(self.generate_blink(t));
            // Schedule the *next* blink 4–10 seconds from now.
            let total = self.current_blink.as_ref().unwrap().total_duration();
            self.next_blink_time = t + total + self.rng.range(4.0, 10.0);
        }
    }

    /// Generate a single blink event starting at time `t`.
    fn generate_blink(&mut self, t: f32) -> BlinkEvent {
        // Total blink duration: 230–350 ms (paper: 7–9 frames @ 30fps, 9 best)
        let total = self.rng.range(0.230, 0.350);
        // Close/open ratio: close 28–38% of total
        let close_ratio = self.rng.range(0.28, 0.38);
        BlinkEvent {
            start_time: t,
            close_duration: total * close_ratio,
            open_duration: total * (1.0 - close_ratio),
        }
    }

    /// Pure computation of eyelid_close at time `t` given current blink state.
    fn compute_value(&self, t: f32) -> f32 {
        let Some(ref blink) = self.current_blink else {
            return self.rest_value;
        };

        let elapsed = t - blink.start_time;

        if elapsed < 0.0 {
            // Before blink starts (peek into the past)
            return self.rest_value;
        }

        if elapsed < blink.close_duration {
            // Closing phase: rest → 1.0 with EaseIn (accelerating shut)
            let p = elapsed / blink.close_duration;
            let eased = apply_easing(p, Easing::EaseIn);
            self.rest_value + (1.0 - self.rest_value) * eased
        } else if elapsed < blink.total_duration() {
            // Opening phase: 1.0 → rest with EaseOut (decelerating open)
            let p = (elapsed - blink.close_duration) / blink.open_duration;
            let eased = apply_easing(p, Easing::EaseOut);
            1.0 - (1.0 - self.rest_value) * eased
        } else {
            // After blink ends
            self.rest_value
        }
    }
}
