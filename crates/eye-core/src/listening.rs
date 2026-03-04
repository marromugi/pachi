/// Voice-driven automatic nod trigger.
///
/// Monitors RMS audio level and detects pauses in speech to trigger
/// head nods as a "listening / understanding" gesture.
///
/// This module is pure logic — it does not perform audio capture.
/// Feed it the current time and smoothed RMS level each frame.

#[derive(Clone, Copy, Debug, PartialEq)]
enum SpeechState {
    /// No recent speech detected.
    Silent,
    /// Speech is ongoing (RMS above threshold).
    Speaking,
    /// Speech just ended; waiting for pause_delay before triggering nod.
    PendingNod { silence_start: f32 },
}

/// Automatic nod trigger driven by microphone audio levels.
pub struct ListeningNod {
    /// RMS level above which audio is considered speech.
    pub speech_threshold: f32,
    /// Seconds of silence after speech before triggering a nod.
    pub pause_delay: f32,
    /// Minimum seconds between consecutive nods.
    pub cooldown: f32,
    /// Whether listening nod detection is enabled.
    pub enabled: bool,

    state: SpeechState,
    last_nod_time: f32,
}

impl Default for ListeningNod {
    fn default() -> Self {
        Self {
            speech_threshold: 0.02,
            pause_delay: 0.4,
            cooldown: 1.5,
            enabled: false,
            state: SpeechState::Silent,
            last_nod_time: f32::NEG_INFINITY,
        }
    }
}

impl ListeningNod {
    /// Feed the current time and RMS audio level.
    /// Returns `true` when a nod should be triggered.
    pub fn update(&mut self, time: f32, rms: f32) -> bool {
        if !self.enabled {
            return false;
        }

        if rms > self.speech_threshold {
            // Voice detected — mark as speaking, cancel any pending nod.
            self.state = SpeechState::Speaking;
            return false;
        }

        // Below threshold — silence.
        match self.state {
            SpeechState::Speaking => {
                // Transition: speech just ended.
                self.state = SpeechState::PendingNod {
                    silence_start: time,
                };
            }
            SpeechState::PendingNod { silence_start } => {
                if time - silence_start >= self.pause_delay {
                    // Pause long enough — check cooldown.
                    self.state = SpeechState::Silent;
                    if time - self.last_nod_time >= self.cooldown {
                        self.last_nod_time = time;
                        return true;
                    }
                }
            }
            SpeechState::Silent => {}
        }

        false
    }

    /// Toggle enabled state and reset internal state.
    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
        self.state = SpeechState::Silent;
    }
}
