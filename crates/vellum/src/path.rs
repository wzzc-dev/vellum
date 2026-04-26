use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::Result;

const APP_STATE_DIR_NAME: &str = "Vellum";
const LAST_OPENED_FILE_NAME: &str = "last-opened.txt";
const RECENT_FILES_NAME: &str = "recent-files.txt";
const MAX_RECENT_FILES: usize = 20;

pub(crate) fn read_last_opened_path() -> Option<PathBuf> {
    let state_file = last_opened_file_path()?;
    let raw = fs::read_to_string(state_file).ok()?;
    parse_last_opened_path(&raw)
}

pub(crate) fn write_last_opened_path(path: &Path) -> Result<()> {
    let Some(state_file) = last_opened_file_path() else {
        return Ok(());
    };

    if let Some(parent) = state_file.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(state_file, path.as_os_str().to_string_lossy().into_owned())?;
    Ok(())
}

pub(crate) fn clear_last_opened_path() {
    if let Some(state_file) = last_opened_file_path() {
        let _ = fs::remove_file(state_file);
    }
}

pub(crate) fn parse_last_opened_path(raw: &str) -> Option<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(PathBuf::from(trimmed))
    }
}

pub(crate) fn app_state_dir() -> Option<PathBuf> {
    let base_dir = if cfg!(target_os = "windows") {
        env::var_os("LOCALAPPDATA")
            .or_else(|| env::var_os("APPDATA"))
            .map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        env::var_os("HOME")
            .map(PathBuf::from)
            .map(|home| home.join("Library").join("Application Support"))
    } else {
        env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
    }?;

    Some(base_dir.join(APP_STATE_DIR_NAME))
}

pub(crate) fn last_opened_file_path() -> Option<PathBuf> {
    app_state_dir().map(|dir| dir.join(LAST_OPENED_FILE_NAME))
}

pub(crate) fn read_recent_files() -> Vec<PathBuf> {
    let Some(dir) = app_state_dir() else {
        return Vec::new();
    };
    let raw = fs::read_to_string(dir.join(RECENT_FILES_NAME)).unwrap_or_default();
    raw.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(PathBuf::from(trimmed))
            }
        })
        .collect()
}

pub(crate) fn write_recent_files(files: &[PathBuf]) -> Result<()> {
    let Some(dir) = app_state_dir() else {
        return Ok(());
    };
    fs::create_dir_all(&dir)?;
    let content = files
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(dir.join(RECENT_FILES_NAME), content)?;
    Ok(())
}

pub(crate) fn add_recent_file(path: &Path) -> Vec<PathBuf> {
    let mut files = read_recent_files();
    files.retain(|p| p != path);
    files.insert(0, path.to_path_buf());
    files.truncate(MAX_RECENT_FILES);
    let _ = write_recent_files(&files);
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_last_opened_path_trims_whitespace() {
        assert_eq!(
            parse_last_opened_path("  /tmp/note.md \n"),
            Some(PathBuf::from("/tmp/note.md"))
        );
    }

    #[test]
    fn parse_last_opened_path_rejects_empty_values() {
        assert_eq!(parse_last_opened_path(" \r\n\t "), None);
    }
}
