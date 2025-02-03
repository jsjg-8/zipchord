mod chordstream;
mod config;
mod text_injector;
use anyhow::{Context, Result};
use config::AppConfig;
use log::{error, info};
use text_injector::TextInjector;
use zipchord::*;

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
    let mut chord_stream = chordstream::ChordStream::new(config.chord_timeout, 1.1)?;

    chord_stream.process_events(|chord| {
        info!("Detected chord: {:?}", chord);

        let expansion = library.resolve(&chord)
                .or_else(|| library.resolve_exception(&chord))
            .or_else(|| library.apply_affixes(&chord));

        if let Some(text) = expansion {
            if let Err(e) = injector.inject_backspaces(chord.len()) {
                eprintln!("Error injecting backspaces: {}", e);
            }
            if let Err(e) = injector.inject(&text) {
                error!("Injection failed: {}", e);
            }
        }
    })?;

    Ok(())
}
