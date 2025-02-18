use anyhow::Result;
use evdev::KeyCode;
use std::time::{Duration, Instant};
use log;

use super::listener::KeyboardListener;
use super::timing::{KeyTiming, TimingAnalyzer};

const MAX_CHORD_SIZE: usize = 8;  // Maximum reasonable number of keys in a chord

pub struct ChordConfig {
    pub base_chord_window: Duration,
    pub roll_threshold: f32,
    pub typing_speed_factor: f32,
    pub min_overlap_ratio: f32,
}

impl Default for ChordConfig {
    fn default() -> Self {
        Self {
            base_chord_window: Duration::from_millis(150),
            roll_threshold: 0.6,
            typing_speed_factor: 0.5,
            min_overlap_ratio: 0.3,
        }
    }
}

#[derive(Debug)]
struct ActiveKey {
    code: KeyCode,
    timing: KeyTiming,
}

pub struct ChordStream {
    active_keys: Vec<ActiveKey>,
    timing_buffer: Vec<KeyTiming>,
    chord_buffer: Vec<KeyCode>,
    last_activity: Instant,
    timing_analyzer: TimingAnalyzer,
    listener: KeyboardListener,
}

impl ChordStream {
    pub fn new(config: ChordConfig) -> Result<Self> {
        Ok(Self {
            active_keys: Vec::with_capacity(MAX_CHORD_SIZE),
            timing_buffer: Vec::with_capacity(MAX_CHORD_SIZE),
            chord_buffer: Vec::with_capacity(MAX_CHORD_SIZE),
            last_activity: Instant::now(),
            timing_analyzer: TimingAnalyzer::new(
                config.base_chord_window,
                config.roll_threshold,
                config.typing_speed_factor,
                config.min_overlap_ratio,
            ),
            listener: KeyboardListener::new()?,
        })
    }

    pub fn with_default_config() -> Result<Self> {
        Self::new(ChordConfig::default())
    }

    pub fn process_events<F>(&mut self, mut callback: F) -> Result<()>
    where
        F: FnMut(Vec<KeyCode>) + 'static,
    {
        let timing_analyzer = &mut self.timing_analyzer;
        let active_keys = &mut self.active_keys;
        let timing_buffer = &mut self.timing_buffer;
        let chord_buffer = &mut self.chord_buffer;
        let last_activity = &mut self.last_activity;

        let chord_callback = move |key: KeyCode, is_press: bool| {
            let event_start = Instant::now();
            let now = Instant::now();
            
            if is_press {
                // Update timing metrics if we have a previous key press
                if let Some(last_key) = active_keys.last() {
                    let interval = now.duration_since(last_key.timing.press_time);
                    timing_analyzer.update_typing_speed(interval);
                }

                // Create new key timing
                let timing = KeyTiming {
                   
                    press_time: now,
                    release_time: None,
                };

                // Check if we should clear existing keys due to timeout
                let chord_window = timing_analyzer.get_adjusted_chord_window();
                if !active_keys.is_empty() {
                    let oldest_press = active_keys[0].timing.press_time;
                    if now.duration_since(oldest_press) > chord_window {
                        active_keys.clear();
                    }
                }

                // Add new key if we haven't exceeded maximum chord size
                if active_keys.len() < MAX_CHORD_SIZE {
                    // Check if key is already in active_keys (shouldn't happen, but let's be safe)
                    if !active_keys.iter().any(|k| k.code == key) {
                        active_keys.push(ActiveKey {
                            code: key,
                            timing,
                        });
                    }
                }
                *last_activity = now;
                
                let event_duration = event_start.elapsed();
                log::debug!("Key press processing took: {:?}", event_duration);
            } else {
                // Key release
                if let Some(pos) = active_keys.iter().position(|k| k.code == key) {
                    active_keys[pos].timing.release_time = Some(now);

                    // Process single key releases immediately
                    if active_keys.len() == 1 {
                        chord_buffer.clear();
                        chord_buffer.push(key);
                        callback(chord_buffer.clone());
                        let event_duration = event_start.elapsed();
                        log::debug!("Single key processing took: {:?}", event_duration);
                    }
                    // Process as potential chord if we have multiple keys
                    else if active_keys.len() > 1 {
                        let chord_detection_start = Instant::now();

                        // Prepare timing buffer
                        timing_buffer.clear();
                        timing_buffer.extend(active_keys.iter().map(|k| k.timing.clone()));
                        
                        // If this forms a valid chord, trigger callback
                        if timing_analyzer.is_chord(timing_buffer) {
                            let chord_start = Instant::now();
                            chord_buffer.clear();
                            chord_buffer.extend(active_keys.iter().map(|k| k.code));
                            
                            log::debug!("Detected chord: {:?}", chord_buffer);
                            if !chord_buffer.is_empty() {
                                callback(chord_buffer.clone());
                                let chord_duration = chord_start.elapsed();
                                let detection_duration = chord_detection_start.elapsed();
                                let total_duration = event_start.elapsed();
                                log::info!("Timing breakdown:");
                                log::info!("  Chord detection: {:?}", detection_duration);
                                log::info!("  Chord injection: {:?}", chord_duration);
                                log::info!("  Total processing: {:?}", total_duration);
                            }
                        } else {
                            log::debug!("Detected roll-over, ignoring sequence");
                            let detection_duration = chord_detection_start.elapsed();
                            log::debug!("Roll-over detection took: {:?}", detection_duration);
                        }
                    }
                    
                    // Remove the released key
                    active_keys.remove(pos);
                }
                
                *last_activity = now;
                
                let event_duration = event_start.elapsed();
                log::debug!("Key release processing took: {:?}", event_duration);
            }
        };

        self.listener.listen(chord_callback)
    }
}

