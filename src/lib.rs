use anyhow::{Context, Result};
use evdev::KeyCode;
use std::{
    collections::HashMap,
    fs,
    path::Path
};

pub mod stream;

pub use stream::ChordStream;

#[derive(Debug, Clone)]
pub struct ChordLibrary {
    pub meta: LibraryMeta,
    pub chords: HashMap<String, String>,
    pub prefixes: HashMap<String, String>, 
    pub suffixes: HashMap<String, String>,
    pub exceptions: HashMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub struct LibraryMeta {
    pub name: String,
    pub language: String,
    pub version: String,
}

impl ChordLibrary {
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;

        let mut parser = LibraryParser::new();
        parser.parse(&content)?;
        
        Ok(parser.into_library())
    }

    pub fn resolve(&self, chord: &[KeyCode]) -> Option<String> {
        let chord_str = self.chord_to_string(chord);
        self.chords.get(&chord_str).cloned()
    }

    pub fn resolve_exception(&self, chord: &[KeyCode]) -> Option<String> {
        let chord_str = self.chord_to_string(chord);
        self.exceptions.get(&chord_str).cloned()
    }

    pub fn apply_affixes(&self, chord: &[KeyCode]) -> Option<String> {
        let chord_str = self.chord_to_string(chord);
        
        // Try prefix first, then suffix
        self.prefixes.get(&chord_str)
            .map(|p| format!("{}_", p))
            .or_else(|| self.suffixes.get(&chord_str)
                .map(|s| format!("_{}", s)))
    }

    fn chord_to_string(&self, chord: &[KeyCode]) -> String {
        // Convert KeyCode to string and sort alphabetically
        let mut keys: Vec<String> = chord.iter()
            .map(|k| format!("{:?}", k))
            .collect();
        keys.sort(); // Sort alphabetically
        keys.join("+")
    }
}

#[derive(Default)]
struct LibraryParser {
    meta: LibraryMeta,
    current_section: Option<Section>,
    chords: HashMap<String, String>,
    prefixes: HashMap<String, String>,
    suffixes: HashMap<String, String>,
    exceptions: HashMap<String, String>,
}

impl LibraryParser {
    fn new() -> Self {
        Self {
            meta: LibraryMeta {
                name: String::new(),
                language: String::new(),
                version: String::new(),
            },
            ..Default::default()
        }
    }

    fn parse(&mut self, content: &str) -> Result<()> {
        for line in content.lines() {
            let line = line.trim();
            
            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse metadata - order matters for metadata
            if let Some(value) = line.strip_prefix("name:") {
                self.meta.name = value.trim().to_string();
            } else if let Some(value) = line.strip_prefix("language:") {
                self.meta.language = value.trim().to_string();
            } else if let Some(value) = line.strip_prefix("version:") {
                self.meta.version = value.trim().to_string();
            }
            // Parse section headers - order doesn't matter
            else if line.starts_with('[') && line.ends_with(']') {
                self.current_section = match line[1..line.len()-1].to_lowercase().as_str() {
                    "prefixes" => Some(Section::Prefix),
                    "suffixes" => Some(Section::Suffix),
                    "chords" => Some(Section::Chord),
                    "exceptions" => Some(Section::Exception),
                    _ => None,
                };
            }
            // Parse mappings - order within sections doesn't matter
            else if let Some((key, value)) = line.split_once("=>") {
                let key = key.trim().to_string();
                // Remove inline comments and trim
                let value = value.split('#').next()
                    .unwrap_or("")
                    .trim()
                    .to_string();
                
                if let Some(section) = &self.current_section {
                    match section {
                        Section::Prefix => { self.prefixes.insert(key, value); }
                        Section::Suffix => { self.suffixes.insert(key, value); }
                        Section::Chord => {
                            // Order doesn't matter for chord keys
                            let mut keys: Vec<&str> = key.split('+').map(str::trim).collect();
                            keys.sort();
                            self.chords.insert(keys.join("+"), value);
                        }
                        Section::Exception => { self.exceptions.insert(key, value); }
                    }
                } else {
                    eprintln!("Warning: Mapping outside section: {}", line);
                }
            }
            // Ignore all other lines
            else {
                eprintln!("Warning: Ignoring line: {}", line);
            }
        }
        
        Ok(())
    }

    fn into_library(self) -> ChordLibrary {
        ChordLibrary {
            meta: self.meta,
            chords: self.chords,
            prefixes: self.prefixes,
            suffixes: self.suffixes,
            exceptions: self.exceptions,
        }
    }
}

#[derive(Debug)]
enum Section {
    Prefix,
    Suffix,
    Chord,
    Exception,
}