mod config;
mod text_injector;
use anyhow::{Context, Result};
use config::AppConfig;
use log::{error, info};
use text_injector::TextInjector;
use zipchord::stream::{ChordStream, ChordConfig};
use zipchord::ChordLibrary;
use evdev::KeyCode;
use std::time::Duration;

fn main() -> Result<()> {
    simple_logger::SimpleLogger::new()
        .init()
        .context("Failed to initialize logger")?;

    info!("Starting ZipChord");

    let config = AppConfig::load()?;
    info!("Loaded config: {:?}", config);

    let library = ChordLibrary::load(&config.library_path.join("english.zc"))?;
    info!("Loaded library: {}", library.meta.name);

    let injector = TextInjector::new()?;

    // Create chord stream with default configuration
    // let mut chord_stream = ChordStream::with_default_config()?;

    // Use custom configuration
    let custom_config = ChordConfig {
        base_chord_window: Duration::from_millis(150),
        roll_threshold: 0.7,
        typing_speed_factor: 0.5,
        min_overlap_ratio: 0.3,
    };
    let mut chord_stream = ChordStream::new(custom_config)?;

    let mut last_char_was_space = true;

    chord_stream.process_events(move |chord| {
        info!("Detected chord: {:?}", chord);

        // Check if the chord contains only a space or punctuation key
        if chord.len() == 1 {
            match chord[0] {
                KeyCode::KEY_SPACE | 
                KeyCode::KEY_DOT | 
                KeyCode::KEY_COMMA |
                KeyCode::KEY_SEMICOLON |
                KeyCode::KEY_APOSTROPHE |
                KeyCode::KEY_GRAVE => {

                        last_char_was_space = true;
                    }
                
                _ => {}
            }
        }

        // Check if we're in the middle of a word
        if !last_char_was_space {
            info!("Ignoring chord in the middle of a word");
            return;
        }

        let expansion = library.resolve(&chord)
                .or_else(|| library.resolve_exception(&chord))
                .or_else(|| library.apply_affixes(&chord));

        if let Some(text) = expansion {
            let text = text.to_string();
            if let Err(e) = injector.inject_backspaces(chord.len()) {
                eprintln!("Error injecting backspaces: {}", e);
            }
            if let Err(e) = injector.inject(&text) {
                error!("Injection failed: {}", e);
            }

            // Update last_char_was_space based on the last character of the injected text
            last_char_was_space = text.chars().last()
                .map(|c| c.is_whitespace() || c == '.' || c == ',' || c == ';' || c == '\'' || c == '`')
                .unwrap_or(false);
        }
    })?;

    Ok(())
}
