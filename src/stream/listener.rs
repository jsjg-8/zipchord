use anyhow::{bail, Result};
use evdev::{Device, EventSummary, KeyCode};
use log::error;
use nix::sys::epoll::{Epoll, EpollCreateFlags, EpollEvent, EpollFlags, EpollTimeout};
use std::os::{
    fd::RawFd,
    unix::io::{AsFd, AsRawFd},
};

pub struct KeyboardListener {
    devices: Vec<Device>,
}

impl KeyboardListener {
    pub fn new() -> Result<Self> {
        let devices = Self::find_keyboards()?;
        if devices.is_empty() {
            bail!("No keyboard devices found");
        }

        Ok(Self { devices })
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
            && device.supported_keys().is_some_and(|keys| {
                keys.contains(KeyCode::KEY_A)
                    && keys.contains(KeyCode::KEY_Z)
                    && keys.contains(KeyCode::KEY_SPACE)
            })
    }

    pub fn listen<F>(&mut self, mut callback: F) -> Result<()>
    where
        F: FnMut(KeyCode, bool), // Callback receives (key, is_press)
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
                    if let EventSummary::Key(_, key, value) = event.destructure() {
                        match value {
                            1 => callback(key, true),  // Key press
                            0 => callback(key, false), // Key release
                            _ => continue,
                        }
                    }
                }
            }
        }
    }
}
