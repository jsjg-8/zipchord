use anyhow::{Context, Result};
use evdev::KeyCode;
use std::{
    collections::HashMap,
    fs,
    path::Path
};

#[derive(Debug)]
pub struct ChordLibrary {
    pub meta: LibraryMeta,
    pub chords: HashMap<String, String>,
    pub prefixes: HashMap<String, String>,
    pub suffixes: HashMap<String, String>,
    pub exceptions: HashMap<String, String>,
}

#[derive(Debug)]
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
        let chord_str = chord.iter()
            .map(|k| format!("{:?}", k))
            .collect::<Vec<_>>()
            .join("+");

        self.chords.get(&chord_str)
            .cloned()
    }

    pub fn resolve_exception(&self, chord: &[KeyCode]) -> Option<String> {
        let chord_str = self.chord_to_string(chord);
        self.exceptions.get(&chord_str).cloned()
    }

    pub fn apply_affixes(&self, chord: &[KeyCode]) -> Option<String> {
        let chord_str = self.chord_to_string(chord);
        
        self.prefixes.get(&chord_str).map(|p| p.clone() + "_")
            .or_else(|| 
                self.suffixes.get(&chord_str).map(|s| "_".to_string() + s)
            )
    }


    fn chord_to_string(&self, chord: &[KeyCode]) -> String {
        chord.iter()
            .map(|k| format!("{:?}", k))
            .collect::<Vec<_>>()
            .join("+")
    }
}

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
            current_section: None,
            chords: HashMap::new(),
            prefixes: HashMap::new(),
            suffixes: HashMap::new(),
            exceptions: HashMap::new(),
        }
    }

    fn parse(&mut self, content: &str) -> Result<()> {
        for line in content.lines() {
            let line = line.trim();
            
            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse metadata
            if line.starts_with("name:") {
                self.meta.name = line.split_once(':')
                    .map(|(_, v)| v.trim().to_string())
                    .unwrap_or_default();
            } else if line.starts_with("language:") {
                self.meta.language = line.split_once(':')
                    .map(|(_, v)| v.trim().to_string())
                    .unwrap_or_default();
            } else if line.starts_with("version:") {
                self.meta.version = line.split_once(':')
                    .map(|(_, v)| v.trim().to_string())
                    .unwrap_or_default();
            }
            // Parse section headers
            else if line.starts_with('[') && line.ends_with(']') {
                self.current_section = match &line[1..line.len()-1].to_lowercase().as_str() {
                    &"prefixes" => Some(Section::Prefix),
                    &"suffixes" => Some(Section::Suffix),
                    &"chords" => Some(Section::Chord),
                    &"exceptions" => Some(Section::Exception),
                    _ => {
                        eprintln!("Warning: Unknown section '{}'", line);
                        None
                    }
                };
            }
            // Parse mappings
            else if let Some((key, value)) = line.split_once("=>") {
                let key = key.trim().to_string();
                let value = value.split('#').next().unwrap().trim().to_string(); // Remove inline comments
                
                if let Some(section) = &self.current_section {
                    match section {
                        Section::Prefix => self.prefixes.insert(key, value),
                        Section::Suffix => self.suffixes.insert(key, value),
                        Section::Chord => {
                            let mut keys: Vec<&str> = key.split('+').map(str::trim).collect();
                            keys.sort();
                            let sorted_key = keys.join("+");
                            self.chords.insert(sorted_key, value)
                        },
                        Section::Exception => self.exceptions.insert(key, value),
                    };
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

enum Section {
    Prefix,
    Suffix,
    Chord,
    Exception,

}