# Chord Keyboard Detector

A Rust application that detects and processes keyboard chord combinations in real-time using Linux's evdev interface.

## Features

- **Real-time Keyboard Monitoring**: Listens to keyboard events using Linux's evdev interface
- **Multi-Device Support**: Automatically detects and monitors all connected keyboard devices
- **Intelligent Chord Detection**: Uses dynamic timing to detect multi-key chord combinations
- **Adaptive Timing**: Adjusts chord detection timing based on user's typing speed using a moving average
- **Non-blocking Operation**: Uses epoll for efficient event handling without blocking
- **Robust Error Handling**: Comprehensive error handling and logging throughout

## Technical Details

### Keyboard Detection
- Automatically identifies keyboard devices by checking for key event support
- Verifies presence of common keys (A, Z, Space) to confirm keyboard functionality
- Supports multiple simultaneous keyboard connections

### Chord Detection Algorithm
- Maintains active key state using precise timestamps
- Dynamic timeout calculation based on user's typing patterns
- Smoothed moving average for inter-key press intervals
- Configurable parameters for timing sensitivity

### Event Processing
- Non-blocking event processing using epoll
- Efficient event batching and processing
- Clean separation between event detection and processing logic

## Usage
