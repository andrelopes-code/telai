use std::collections::HashMap;
use std::path::{Path, PathBuf};

use wasapi::{DeviceEnumerator, Direction, SessionState};

use crate::process::{ProcessTree, image_path};

/// About -60 dBFS: quieter sessions count as silent.
const AUDIBLE_PEAK: f32 = 0.001;

/// Ids of the processes whose audio sessions are playing sound right now, on
/// any active output device.
///
/// Initializes COM for the calling thread in the multithreaded apartment, so
/// it must not run on a UI thread.
pub fn audible_processes() -> Result<Vec<u32>, crate::Error> {
    wasapi::initialize_mta().ok()?;

    let devices = DeviceEnumerator::new()?.get_device_collection(Direction::Render)?;

    let mut audible = Vec::new();
    for device in &devices {
        let sessions = device?
            .get_iaudiosessionmanager()?
            .get_audiosessionenumerator()?;

        for index in 0..sessions.get_count()? {
            let session = sessions.get_session(index)?;
            if session.get_state()? != SessionState::Active {
                continue;
            }

            let peak = session.get_audiometerinformation()?.get_peak_value()?;
            if peak > AUDIBLE_PEAK {
                audible.push(session.get_process_id()?);
            }
        }
    }

    Ok(audible)
}

/// An app with an audio session open on an output device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioApp {
    /// The root of the app's process tree: capturing it with its children
    /// records every sound the app makes.
    pub process_id: u32,
    pub executable: PathBuf,
    /// Executable name without its extension, as written on disk.
    pub name: String,
}

/// Apps with an active audio session on any output device, playing sound or
/// not, sorted by name. The system sounds session and this process are left
/// out.
pub fn audio_apps() -> Result<Vec<AudioApp>, crate::Error> {
    let session_processes = active_session_processes()?;
    let tree = ProcessTree::snapshot()?;
    let own_process = std::process::id();

    let mut executables: HashMap<u32, Option<PathBuf>> = HashMap::new();
    let mut executable_of = |process_id: u32| {
        executables
            .entry(process_id)
            .or_insert_with(|| image_path(process_id))
            .clone()
    };

    let mut apps: Vec<AudioApp> = Vec::new();
    for process_id in session_processes {
        if process_id == 0 || process_id == own_process {
            continue;
        }

        let root = app_root(process_id, &tree, &mut executable_of);
        if apps.iter().any(|app| app.process_id == root) {
            continue;
        }

        let Some(executable) = executable_of(root) else {
            continue;
        };
        let name = executable
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default();
        apps.push(AudioApp {
            process_id: root,
            executable,
            name,
        });
    }

    apps.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then(a.process_id.cmp(&b.process_id))
    });

    Ok(apps)
}

fn active_session_processes() -> Result<Vec<u32>, crate::Error> {
    wasapi::initialize_mta().ok()?;

    let devices = DeviceEnumerator::new()?.get_device_collection(Direction::Render)?;

    let mut processes = Vec::new();
    for device in &devices {
        let sessions = device?
            .get_iaudiosessionmanager()?
            .get_audiosessionenumerator()?;

        for index in 0..sessions.get_count()? {
            let session = sessions.get_session(index)?;
            if session.get_state()? == SessionState::Active {
                processes.push(session.get_process_id()?);
            }
        }
    }

    Ok(processes)
}

/// Deepest ancestor of `process_id` reached through parents that run the
/// same executable, such as the main process of a browser or of Discord.
fn app_root(
    process_id: u32,
    tree: &ProcessTree,
    mut executable_of: impl FnMut(u32) -> Option<PathBuf>,
) -> u32 {
    let Some(executable) = executable_of(process_id) else {
        return process_id;
    };

    let mut root = process_id;
    // Process ids are reused, so parent links can loop; the bound ends the
    // walk in that case.
    for _ in 0..MAX_DEPTH {
        let Some(parent) = tree.parent(root) else {
            break;
        };
        match executable_of(parent) {
            Some(path) if same_executable(&path, &executable) => root = parent,
            _ => break,
        }
    }

    root
}

/// Deeper than any real chain of processes running one executable.
const MAX_DEPTH: usize = 64;

fn same_executable(a: &Path, b: &Path) -> bool {
    a.to_string_lossy()
        .eq_ignore_ascii_case(&b.to_string_lossy())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::ProcessTree;

    #[test]
    fn a_child_with_the_same_executable_belongs_to_its_root() {
        // 30 (Discord voice) -> 20 (Discord) -> 10 (explorer)
        let tree = ProcessTree::from_links(&[(30, 20), (20, 10), (10, 1)]);
        let exe = |id| {
            Some(PathBuf::from(match id {
                30 | 20 => r"C:\Discord\Discord.exe",
                _ => r"C:\Windows\explorer.exe",
            }))
        };

        assert_eq!(app_root(30, &tree, exe), 20);
    }

    #[test]
    fn a_process_whose_parent_runs_another_executable_is_its_own_root() {
        let tree = ProcessTree::from_links(&[(30, 20)]);
        let exe = |id| {
            Some(PathBuf::from(if id == 30 {
                "vlc.exe"
            } else {
                "explorer.exe"
            }))
        };

        assert_eq!(app_root(30, &tree, exe), 30);
    }

    #[test]
    fn the_executable_comparison_ignores_case() {
        let tree = ProcessTree::from_links(&[(30, 20)]);
        let exe = |id| {
            Some(PathBuf::from(if id == 30 {
                r"C:\Program Files\Brave\brave.exe"
            } else {
                r"c:\program files\brave\BRAVE.EXE"
            }))
        };

        assert_eq!(app_root(30, &tree, exe), 20);
    }
}
