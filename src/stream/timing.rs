use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct KeyTiming {
    pub press_time: Instant,
    pub release_time: Option<Instant>,
}

#[derive(Debug, Clone)]
pub struct TimingAnalyzer {
    // Configuration
    base_chord_window: Duration,
    roll_threshold: f32,
    typing_speed_factor: f32,
    min_overlap_ratio: f32,
    base_window_secs: f32, // Cached conversion

    // State
    recent_press_intervals: Vec<Duration>,
    average_typing_speed: Duration,
    cached_chord_window: Duration,
    last_speed_update: Instant,
    speed_cache_duration: Duration,
}

impl TimingAnalyzer {
    pub fn new(
        base_chord_window: Duration,
        roll_threshold: f32,
        typing_speed_factor: f32,
        min_overlap_ratio: f32,
    ) -> Self {
        Self {
            base_chord_window,
            roll_threshold,
            typing_speed_factor,
            min_overlap_ratio,
            base_window_secs: base_chord_window.as_secs_f32(),
            recent_press_intervals: Vec::with_capacity(10),
            average_typing_speed: base_chord_window,
            cached_chord_window: base_chord_window,
            last_speed_update: Instant::now(),
            speed_cache_duration: Duration::from_millis(100), // Update cache every 100ms
        }
    }

    /// Calculate a roll-over score for a sequence of key events
    /// Returns a score between 0.0 and 1.0, where:
    /// - 0.0 indicates perfect chord (simultaneous presses)
    /// - 1.0 indicates clear roll-over (sequential presses)
    pub fn calculate_roll_score(&self, timings: &[KeyTiming]) -> f32 {
        if timings.len() < 2 {
            return 0.0;
        }

        let mut total_score = 0.0;
        let mut count = 0;

        // Avoid sorting by using windows directly - timings should already be ordered
        for window in timings.windows(2) {
            let current = &window[0];
            let next = &window[1];

            // Calculate time between presses
            let press_interval = next.press_time.duration_since(current.press_time);
            let press_interval_secs = press_interval.as_secs_f32();

            // Fast path for clearly sequential presses
            if press_interval_secs > self.base_window_secs {
                total_score += 1.0;
                count += 1;
                continue;
            }

            // Calculate overlap
            let overlap_secs = if let Some(release_time) = current.release_time {
                if release_time > next.press_time {
                    release_time.duration_since(next.press_time).as_secs_f32()
                } else {
                    0.0
                }
            } else {
                self.base_window_secs // If no release time, assume full overlap
            };

            // Calculate overlap ratio
            let overlap_ratio = overlap_secs / self.base_window_secs;

            if overlap_ratio < self.min_overlap_ratio {
                total_score += 1.0;
                count += 1;
                continue;
            }

            // Calculate press interval ratio
            let interval_ratio = press_interval_secs / self.base_window_secs;

            // Combine factors into a score
            total_score += 1.0 - (overlap_ratio * (1.0 - interval_ratio));
            count += 1;
        }

        let base_score = if count > 0 {
            total_score / count as f32
        } else {
            0.0
        };
        self.adjust_for_typing_speed(base_score)
    }

    /// Update typing speed metrics with a new interval
    pub fn update_typing_speed(&mut self, interval: Duration) {
        const MAX_SAMPLES: usize = 10;

        self.recent_press_intervals.push(interval);
        if self.recent_press_intervals.len() > MAX_SAMPLES {
            self.recent_press_intervals.remove(0);
        }

        let now = Instant::now();
        if now.duration_since(self.last_speed_update) >= self.speed_cache_duration {
            // Update average typing speed
            if !self.recent_press_intervals.is_empty() {
                let sum: Duration = self.recent_press_intervals.iter().sum();
                self.average_typing_speed = sum / self.recent_press_intervals.len() as u32;
            }

            // Update cached chord window
            let speed_ratio = self.average_typing_speed.as_secs_f32() / self.base_window_secs;
            let adjustment = 1.0 + (speed_ratio - 1.0) * self.typing_speed_factor;
            self.cached_chord_window = self.base_chord_window.mul_f32(adjustment.clamp(0.5, 2.0));

            self.last_speed_update = now;
        }
    }

    /// Get the current chord window based on typing speed
    pub fn get_adjusted_chord_window(&self) -> Duration {
        self.cached_chord_window
    }

    /// Determine if a sequence of key timings represents a chord
    pub fn is_chord(&self, timings: &[KeyTiming]) -> bool {
        let roll_score = self.calculate_roll_score(timings);
        roll_score < self.roll_threshold
    }

    fn adjust_for_typing_speed(&self, score: f32) -> f32 {
        let typing_speed_ratio = self.average_typing_speed.as_secs_f32() / self.base_window_secs;
        let adjustment = 1.0 - (typing_speed_ratio - 1.0) * self.typing_speed_factor;
        (score * adjustment).clamp(0.0, 1.0)
    }
}
