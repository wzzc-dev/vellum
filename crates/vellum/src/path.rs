use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::Result;

const APP_STATE_DIR_NAME: &str = "Vellum";
const LAST_OPENED_FILE_NAME: &str = "last-opened.txt";

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

pub(crate) fn last_opened_file_path() -> Option<PathBuf> {
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

    Some(
        base_dir
            .join(APP_STATE_DIR_NAME)
            .join(LAST_OPENED_FILE_NAME),
    )
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
