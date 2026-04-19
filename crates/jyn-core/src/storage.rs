// Copyright (c) 2026 Joydev GmbH (joydev.com)
// SPDX-License-Identifier: MIT

//! Workspace storage for jot.
//!
//! Jot reuses joy-core's generic YAML primitives (`store::write_yaml`,
//! `store::read_yaml`) and ID helpers (`item_filename`, `title_hash_suffix`)
//! to operate on `.jot/` without duplicating IO logic or requiring changes
//! to joy-core. See `docs/dev/Architecture.md`.

use std::path::{Path, PathBuf};

use joy_core::items::title_hash_suffix;
use joy_core::model::item::item_filename;
use joy_core::store::{read_yaml, write_yaml};

use crate::error::JotError;
use crate::model::Task;

pub const JOT_DIR: &str = ".jot";
pub const ITEMS_DIR: &str = "items";
pub const ACRONYM: &str = "TODO";

pub fn jot_dir(root: &Path) -> PathBuf {
    root.join(JOT_DIR)
}

pub fn items_dir(root: &Path) -> PathBuf {
    jot_dir(root).join(ITEMS_DIR)
}

/// Create `.jot/items/` if missing. Idempotent.
pub fn ensure_items_dir(root: &Path) -> Result<(), JotError> {
    let dir = items_dir(root);
    std::fs::create_dir_all(&dir)
        .map_err(|e| JotError::Other(format!("cannot create {}: {}", dir.display(), e)))
}

/// Write a task to `.jot/items/{ID}-{slug}.yaml`.
pub fn save_task(root: &Path, task: &Task) -> Result<(), JotError> {
    ensure_items_dir(root)?;
    let filename = item_filename(&task.item.id, &task.item.title);
    let path = items_dir(root).join(filename);
    write_yaml(&path, task)?;
    Ok(())
}

/// Load all tasks from `.jot/items/`, sorted by filename.
pub fn load_tasks(root: &Path) -> Result<Vec<Task>, JotError> {
    let dir = items_dir(root);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .map_err(|e| JotError::Other(format!("cannot read {}: {}", dir.display(), e)))?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|ext| ext == "yaml" || ext == "yml")
        })
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let mut tasks = Vec::with_capacity(entries.len());
    for entry in entries {
        let task: Task = read_yaml(&entry.path())?;
        tasks.push(task);
    }
    Ok(tasks)
}

/// Find the file for a task ID. Accepts the display short form (`#A1`,
/// `A1`), the ADR-027 short form (`TODO-00A1`), or the full form
/// (`TODO-00A1-EA`). Returns an error if the ID is ambiguous or missing.
pub fn find_task_file(root: &Path, id: &str) -> Result<PathBuf, JotError> {
    let dir = items_dir(root);
    if !dir.is_dir() {
        return Err(JotError::Other(format!("task not found: {id}")));
    }
    let normalized = crate::display::normalize_id_input(id);
    let prefix = format!("{normalized}-");

    let matches: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map_err(|e| JotError::Other(format!("cannot read {}: {}", dir.display(), e)))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .map(|n| n.to_string_lossy().to_uppercase().starts_with(&prefix))
                .unwrap_or(false)
        })
        .collect();

    match matches.len() {
        0 => Err(JotError::Other(format!("task not found: {id}"))),
        1 => Ok(matches.into_iter().next().unwrap()),
        _ => Err(JotError::Other(format!(
            "ambiguous ID {id}: {} matches",
            matches.len()
        ))),
    }
}

/// Load a single task by its full or short ID.
pub fn load_task(root: &Path, id: &str) -> Result<Task, JotError> {
    let path = find_task_file(root, id)?;
    Ok(read_yaml(&path)?)
}

/// Overwrite a task on disk. If the title changed and produced a new
/// filename (slug-derived), the old file is removed.
pub fn update_task(root: &Path, task: &Task) -> Result<(), JotError> {
    let old_path = find_task_file(root, &task.item.id)?;
    save_task(root, task)?;
    let new_path = items_dir(root).join(item_filename(&task.item.id, &task.item.title));
    if old_path != new_path {
        let _ = std::fs::remove_file(&old_path);
    }
    Ok(())
}

/// Delete a task by ID. Returns the deleted task.
pub fn delete_task(root: &Path, id: &str) -> Result<Task, JotError> {
    let path = find_task_file(root, id)?;
    let task: Task = read_yaml(&path)?;
    std::fs::remove_file(&path)
        .map_err(|e| JotError::Other(format!("cannot remove {}: {}", path.display(), e)))?;
    Ok(task)
}

/// Generate the next ID in the form `TODO-XXXX-YY` (ADR-027).
pub fn next_id(root: &Path, title: &str) -> Result<String, JotError> {
    let suffix = title_hash_suffix(title);
    let dir = items_dir(root);
    if !dir.is_dir() {
        return Ok(format!("{ACRONYM}-0001-{suffix}"));
    }

    let prefix = format!("{ACRONYM}-");
    let mut max_num: u16 = 0;
    for entry in std::fs::read_dir(&dir)
        .map_err(|e| JotError::Other(format!("cannot read {}: {}", dir.display(), e)))?
        .filter_map(|e| e.ok())
    {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if let Some(rest) = name.strip_prefix(&prefix) {
            if let Some(hex) = rest.get(..4) {
                if let Ok(n) = u16::from_str_radix(hex, 16) {
                    max_num = max_num.max(n);
                }
            }
        }
    }

    let next = max_num.checked_add(1).ok_or_else(|| {
        JotError::Other(format!("{ACRONYM} ID space exhausted (max {ACRONYM}-FFFF)"))
    })?;
    Ok(format!("{ACRONYM}-{next:04X}-{suffix}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_task(id: &str, title: &str) -> Task {
        Task::new(id.into(), title.into())
    }

    #[test]
    fn next_id_first_in_empty_dir() {
        let dir = tempdir().unwrap();
        let id = next_id(dir.path(), "Buy milk").unwrap();
        assert!(id.starts_with("TODO-0001-"), "got: {id}");
        assert_eq!(id.len(), 12);
    }

    #[test]
    fn next_id_increments_after_save() {
        let dir = tempdir().unwrap();
        let t1 = make_task("TODO-0001-A3", "First");
        save_task(dir.path(), &t1).unwrap();
        let id2 = next_id(dir.path(), "Second").unwrap();
        assert!(id2.starts_with("TODO-0002-"), "got: {id2}");
    }

    #[test]
    fn save_then_load_roundtrip() {
        let dir = tempdir().unwrap();
        let id = next_id(dir.path(), "Buy milk").unwrap();
        let task = make_task(&id, "Buy milk");
        save_task(dir.path(), &task).unwrap();
        let loaded = load_tasks(dir.path()).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].item.id, id);
        assert_eq!(loaded[0].item.title, "Buy milk");
    }

    #[test]
    fn load_tasks_empty_returns_empty() {
        let dir = tempdir().unwrap();
        let loaded = load_tasks(dir.path()).unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn delete_removes_file() {
        let dir = tempdir().unwrap();
        let id = next_id(dir.path(), "Temp").unwrap();
        let task = make_task(&id, "Temp");
        save_task(dir.path(), &task).unwrap();
        let deleted = delete_task(dir.path(), &id).unwrap();
        assert_eq!(deleted.item.id, id);
        assert!(load_tasks(dir.path()).unwrap().is_empty());
    }

    #[test]
    fn find_task_file_short_form() {
        let dir = tempdir().unwrap();
        let id = next_id(dir.path(), "Short form").unwrap();
        let task = make_task(&id, "Short form");
        save_task(dir.path(), &task).unwrap();
        let short = &id[..9]; // "TODO-0001"
        let path = find_task_file(dir.path(), short).unwrap();
        assert!(path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with(short));
    }

    #[test]
    fn find_task_file_missing_errors() {
        let dir = tempdir().unwrap();
        ensure_items_dir(dir.path()).unwrap();
        let err = find_task_file(dir.path(), "TODO-9999");
        assert!(err.is_err());
    }
}
