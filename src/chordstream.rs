use anyhow::{bail, Result};
use evdev::{Device, EventSummary, InputEvent, KeyCode};
use log::error;
use nix::sys::epoll::{Epoll, EpollCreateFlags, EpollEvent, EpollFlags, EpollTimeout};
use std::{
    collections::HashMap,
    os::{
        fd::RawFd,
        unix::io::{AsFd, AsRawFd},
    },
    time::{Duration, Instant},
};

pub struct ChordStream {
    devices: Vec<Device>,
    active_keys: HashMap<KeyCode, Instant>,
    last_activity: Instant,
    chord_timeout_base: Duration, // Base timeout, will be dynamically adjusted
    dynamic_timeout_multiplier: f32, // Multiplier for dynamic timeout
    inter_press_intervals: Vec<Duration>, // Store recent inter-press intervals
    inter_press_average: Duration, // Moving average of inter-press intervals
    last_key_press: Option<Instant>,
}

impl ChordStream {
    pub fn new(timeout_base: Duration, dynamic_timeout_multiplier: f32) -> Result<Self> {
        let devices = Self::find_keyboards()?;
        if devices.is_empty() {
            bail!("No keyboard devices found");
        }

        Ok(Self {
            devices,
            active_keys: HashMap::new(),
            last_activity: Instant::now(),
            chord_timeout_base: timeout_base,  // Use base timeout
            dynamic_timeout_multiplier,        // Store multiplier
            inter_press_intervals: Vec::new(), // Initialize intervals
            inter_press_average: timeout_base, // Initialize average with base timeout
            last_key_press: None,
        })
    }

    fn find_keyboards() -> Result<Vec<Device>> {
        let mut keyboards = Vec::new();

        for (path, device) in evdev::enumerate() {
            if Self::is_keyboard(&device) {
                println!(
                    "Using keyboard: {} ({})",
                    device.name().unwrap_or("Unknown"),
                    path.display()
                );
                keyboards.push(device);
            }
        }

        Ok(keyboards)
    }

    fn is_keyboard(device: &Device) -> bool {
        device.supported_events().contains(evdev::EventType::KEY)
            && device.supported_keys().map_or(false, |keys| {
                keys.contains(KeyCode::KEY_A)
                    && keys.contains(KeyCode::KEY_Z)
                    && keys.contains(KeyCode::KEY_SPACE)
            })
    }

    pub fn process_events<F>(&mut self, mut callback: F) -> Result<()>
    where
        F: FnMut(Vec<KeyCode>),
    {
        let epoll = Epoll::new(EpollCreateFlags::empty())?;

        // Store raw file descriptors alongside devices
        let device_fds: Vec<(RawFd, &mut Device)> = self
            .devices
            .iter_mut()
            .map(|d| (d.as_raw_fd(), d))
            .collect();

        // Add all devices to epoll and set non-blocking
        for (fd, dev) in &device_fds {
            dev.set_nonblocking(true)?;
            epoll.add(
                dev.as_fd(),
                EpollEvent::new(EpollFlags::EPOLLIN, *fd as u64),
            )?;
        }

        let mut events = vec![EpollEvent::empty(); device_fds.len()];

        loop {
            let num_events = epoll.wait(&mut events, EpollTimeout::NONE)?;

            for event in events.iter().take(num_events) {
                let fd = event.data() as RawFd;

                let events = {
                    // Short-lived device borrow
                    let device = match self.devices.iter_mut().find(|d| d.as_raw_fd() == fd) {
                        Some(d) => d,
                        None => continue,
                    };

                    match device.fetch_events() {
                        Ok(events_iter) => events_iter.into_iter().collect(),
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            vec![]
                        }
                        Err(e) => {
                            error!("Error reading events: {}", e);
                            vec![]
                        }
                    }
                };

                // Process the fetched events
                for event in events {
                    self.process_event(event, &mut callback);
                }
            }
        }
    }

    fn process_event<F>(&mut self, event: InputEvent, callback: &mut F)
    where
        F: FnMut(Vec<KeyCode>),
    {
        if let EventSummary::Key(_, key, value) = event.destructure() {
            println!("Event: Key: {:?}, Value: {}", key, value);
            match value {
                1 => {
                    // Key Press
                    self.handle_key_press(key, callback);
                }
                0 => {
                    // Key Release
                    self.handle_key_release(key, callback);
                }
                _ => return,
            };
            self.last_activity = Instant::now();
        }
    }

    fn handle_key_press<F>(&mut self, key: KeyCode, callback: &mut F)
    where
        F: FnMut(Vec<KeyCode>),
    {
        let now = Instant::now();
        println!("handle_key_press: Key: {:?}", key);
        if let Some(last_press_time) = self.last_key_press {
            let inter_press_duration = now.duration_since(last_press_time);
            if inter_press_duration > self.get_chord_timeout() && !self.active_keys.is_empty() {
                // Timeout between key presses, clear existing active keys if timed out
                self.active_keys.clear();
            }
            self.update_inter_press_average(inter_press_duration); // Update moving average
        }
        self.active_keys.insert(key, now);
        self.last_key_press = Some(now);
    }

    fn handle_key_release<F>(&mut self, key: KeyCode, callback: &mut F) -> Option<Vec<KeyCode>>
    where
        F: FnMut(Vec<KeyCode>),
    {
        println!("handle_key_release: Key: {:?}", key);
        self.active_keys.remove(&key);
        println!("  Active keys after remove: {:?}", self.active_keys.keys());
        let mut chord: Vec<_> = self.active_keys.keys().cloned().collect();
        chord.push(key); // Add the released key
        callback(chord.clone());
        return Some(chord);
    }

    fn get_chord_timeout(&self) -> Duration {
        // Dynamic timeout based on moving average and multiplier
        self.inter_press_average
            .mul_f32(self.dynamic_timeout_multiplier)
    }

    fn update_inter_press_average(&mut self, interval: Duration) {
        const INTERVAL_HISTORY_SIZE: usize = 10; // Number of intervals to average
        const SMOOTHING_FACTOR: f32 = 0.1; // Smoothing factor for moving average

        // Keep a history of recent inter-press intervals
        self.inter_press_intervals.push(interval);
        if self.inter_press_intervals.len() > INTERVAL_HISTORY_SIZE {
            self.inter_press_intervals.remove(0); // Keep only the last INTERVAL_HISTORY_SIZE intervals
        }

        // Recalculate moving average
        let sum: Duration = self.inter_press_intervals.iter().sum();
        let count = self.inter_press_intervals.len() as u32;

        if count > 0 {
            let new_average = sum / count;
            // Apply smoothing to the average
            self.inter_press_average = self.inter_press_average.mul_f32(1.0 - SMOOTHING_FACTOR)
                + new_average.mul_f32(SMOOTHING_FACTOR);
        }
    }
}
