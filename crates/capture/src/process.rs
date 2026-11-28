use std::collections::HashMap;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::core::PWSTR;

/// Long enough for any path, including the `\\?\` long-path form.
const PATH_CAPACITY: u32 = 32_768;

#[allow(
    clippy::cast_possible_truncation,
    reason = "the entry is a few hundred bytes"
)]
const ENTRY_SIZE: u32 = size_of::<PROCESSENTRY32W>() as u32;

/// Parent links of every running process, taken at one point in time.
pub struct ProcessTree {
    parents: HashMap<u32, u32>,
}

impl ProcessTree {
    pub fn snapshot() -> windows::core::Result<Self> {
        let snapshot = OwnedHandle(unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)? });
        let mut entry = PROCESSENTRY32W {
            dwSize: ENTRY_SIZE,
            ..Default::default()
        };

        let mut parents = HashMap::new();
        let mut next = unsafe { Process32FirstW(snapshot.0, &raw mut entry) };
        while next.is_ok() {
            parents.insert(entry.th32ProcessID, entry.th32ParentProcessID);
            next = unsafe { Process32NextW(snapshot.0, &raw mut entry) };
        }

        Ok(Self { parents })
    }

    /// Whether `process_id` is `root` or one of its descendants.
    #[must_use]
    pub fn is_within(&self, process_id: u32, root: u32) -> bool {
        let mut current = process_id;

        // Process ids are reused, so a parent chain can loop; the depth bound
        // ends the walk in that case.
        for _ in 0..self.parents.len() {
            if current == root {
                return true;
            }

            match self.parents.get(&current) {
                Some(&parent) if parent != current && parent != 0 => current = parent,
                _ => return false,
            }
        }

        false
    }

    /// The parent of `process_id`, unless it is unknown, the system idle
    /// process or the process itself.
    #[must_use]
    pub fn parent(&self, process_id: u32) -> Option<u32> {
        self.parents
            .get(&process_id)
            .copied()
            .filter(|&parent| parent != 0 && parent != process_id)
    }

    /// A tree from (child, parent) links, for tests.
    #[cfg(test)]
    pub(crate) fn from_links(links: &[(u32, u32)]) -> Self {
        Self {
            parents: links.iter().copied().collect(),
        }
    }
}

/// Full path of the executable of a running process.
#[must_use]
pub fn image_path(process_id: u32) -> Option<PathBuf> {
    let process =
        unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) }.ok()?;
    let process = OwnedHandle(process);

    let mut buffer = vec![0u16; PATH_CAPACITY as usize];
    let mut length = PATH_CAPACITY;
    let name = PWSTR(buffer.as_mut_ptr());
    unsafe { QueryFullProcessImageNameW(process.0, PROCESS_NAME_WIN32, name, &raw mut length) }
        .ok()?;

    let path = OsString::from_wide(&buffer[..length as usize]);

    Some(PathBuf::from(path))
}

struct OwnedHandle(HANDLE);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        // Closing a handle this type opened cannot fail in a recoverable way.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_descendant_is_within_its_ancestor() {
        let tree = tree(&[(30, 20), (20, 10), (10, 1)]);

        assert!(tree.is_within(30, 10));
        assert!(tree.is_within(10, 10));
    }

    #[test]
    fn a_sibling_is_not_within_another_branch() {
        let tree = tree(&[(30, 20), (40, 10), (20, 1)]);

        assert!(!tree.is_within(40, 20));
    }

    #[test]
    fn a_parent_cycle_ends_the_walk() {
        let tree = tree(&[(10, 20), (20, 10)]);

        assert!(!tree.is_within(10, 99));
    }

    #[test]
    fn the_current_process_is_within_its_parent_in_a_snapshot() {
        let tree = ProcessTree::snapshot().unwrap();
        let own = std::process::id();
        let parent = tree.parents[&own];

        assert!(tree.is_within(own, parent));
    }

    fn tree(links: &[(u32, u32)]) -> ProcessTree {
        ProcessTree::from_links(links)
    }
}
