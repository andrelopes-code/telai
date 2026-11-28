//! Which apps the screen stream carries, and how loud, remembered between
//! runs. Apps are named by their executable without the extension, in lower
//! case, so every process of an app shares one choice.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};

/// Voice chat apps: their sound is the other people in the call, who would
/// hear themselves back.
const EXCLUDED_BY_DEFAULT: [&str; 3] = ["discord", "discordptb", "discordcanary"];

/// The file name, inside the app config directory.
pub const FILE_NAME: &str = "audio-mix.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioPrefs {
    excluded: BTreeSet<String>,
    volumes: BTreeMap<String, f32>,
}

impl Default for AudioPrefs {
    fn default() -> Self {
        Self {
            excluded: EXCLUDED_BY_DEFAULT.map(String::from).into(),
            volumes: BTreeMap::new(),
        }
    }
}

impl AudioPrefs {
    pub fn is_included(&self, name: &str) -> bool {
        !self.excluded.contains(&key(name))
    }

    /// From 0 to 1; 1 for an app never adjusted.
    pub fn volume(&self, name: &str) -> f32 {
        self.volumes.get(&key(name)).copied().unwrap_or(1.0)
    }

    pub fn set(&mut self, name: &str, included: bool, volume: f32) {
        let key = key(name);
        if included {
            self.excluded.remove(&key);
        } else {
            self.excluded.insert(key.clone());
        }

        let volume = if volume.is_nan() {
            1.0
        } else {
            volume.clamp(0.0, 1.0)
        };
        self.volumes.insert(key, volume);
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }

        // A file written in place could be left half written; a renamed one
        // is either the old file or the new one.
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        let temporary = path.with_extension("json.tmp");
        std::fs::write(&temporary, json)?;
        std::fs::rename(&temporary, path)
    }
}

/// Applies one choice and saves the result while still holding the lock, so
/// that saves never overlap or land out of order. Returns the new choices.
pub fn update(
    prefs: &Mutex<AudioPrefs>,
    path: Option<&Path>,
    name: &str,
    included: bool,
    volume: f32,
) -> AudioPrefs {
    let mut prefs = prefs.lock().unwrap_or_else(PoisonError::into_inner);
    prefs.set(name, included, volume);

    if let Some(path) = path
        && let Err(error) = prefs.save(path)
    {
        tracing::warn!(%error, "the audio choices could not be saved");
    }

    prefs.clone()
}

/// The saved choices, or the defaults when there are none yet or the file
/// cannot be read.
pub fn load(path: &Path) -> AudioPrefs {
    let Ok(json) = std::fs::read_to_string(path) else {
        return AudioPrefs::default();
    };

    serde_json::from_str(&json).unwrap_or_else(|error| {
        tracing::warn!(%error, path = %path.display(), "the audio choices could not be read");
        AudioPrefs::default()
    })
}

fn key(name: &str) -> String {
    name.to_lowercase()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join("telai-audio-prefs-test")
            .join(format!("{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn discord_is_excluded_by_default() {
        let prefs = AudioPrefs::default();

        assert!(!prefs.is_included("discord"));
        assert!(!prefs.is_included("Discord"));
        assert!(!prefs.is_included("DiscordPTB"));
        assert!(!prefs.is_included("DiscordCanary"));
        assert!(prefs.is_included("vlc"));
        assert!(close(prefs.volume("vlc"), 1.0));
    }

    #[test]
    fn a_missing_or_corrupt_file_gives_the_defaults() {
        let dir = temp_dir("corrupt");
        assert!(!load(&dir.join("absent.json")).is_included("discord"));

        std::fs::write(dir.join("bad.json"), "{not json").unwrap();
        assert!(!load(&dir.join("bad.json")).is_included("discord"));
    }

    #[test]
    fn choices_survive_a_save_and_load() {
        let path = temp_dir("roundtrip").join("nested").join("audio-mix.json");
        let mut prefs = AudioPrefs::default();
        prefs.set("Discord", true, 0.3);
        prefs.set("vlc", false, 1.5);
        prefs.save(&path).unwrap();

        let loaded = load(&path);

        assert!(loaded.is_included("discord"));
        assert!(close(loaded.volume("DISCORD"), 0.3));
        assert!(!loaded.is_included("vlc"));
        assert!(close(loaded.volume("vlc"), 1.0));
    }
}

#[cfg(test)]
mod concurrency_tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    #[test]
    fn overlapping_changes_leave_a_valid_file_with_the_last_state() {
        let dir = std::env::temp_dir()
            .join("telai-audio-prefs-test")
            .join(format!("concurrent-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("audio-mix.json");
        let prefs = Arc::new(Mutex::new(AudioPrefs::default()));

        let threads: Vec<_> = (0..8)
            .map(|thread| {
                let prefs = Arc::clone(&prefs);
                let path = path.clone();
                std::thread::spawn(move || {
                    for step in 0..40 {
                        let name = format!("app-with-a-longer-name-{thread}-{step}");
                        update(&prefs, Some(&path), &name, step % 2 == 0, 0.5);
                    }
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }

        let json = std::fs::read_to_string(&path).unwrap();
        let saved: AudioPrefs = serde_json::from_str(&json).expect("the file is valid JSON");
        let in_memory = prefs.lock().unwrap().clone();
        assert_eq!(saved.volumes, in_memory.volumes);
        assert_eq!(saved.excluded, in_memory.excluded);
        assert!(!saved.is_included("discord"));
        assert!(!dir.join("audio-mix.json.tmp").exists());
    }
}
