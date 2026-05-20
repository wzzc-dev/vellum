use std::fs;

use anyhow::Result;
use editor::SyntaxTheme;

use crate::path::preferences_file_path;

#[derive(Debug, Clone)]
pub(super) struct AppPreferences {
    pub syntax_theme: SyntaxTheme,
    pub sidebar_visible: bool,
    pub status_bar_pinned: bool,
    pub focus_mode: bool,
    pub typewriter_mode: bool,
    pub focus_highlight_mode: bool,
    pub font_size: u16,
}

impl Default for AppPreferences {
    fn default() -> Self {
        Self {
            syntax_theme: SyntaxTheme::Default,
            sidebar_visible: true,
            status_bar_pinned: false,
            focus_mode: false,
            typewriter_mode: false,
            focus_highlight_mode: false,
            font_size: 17,
        }
    }
}

pub(super) fn load_preferences() -> AppPreferences {
    let Some(path) = preferences_file_path() else {
        return AppPreferences::default();
    };
    let Ok(raw) = fs::read_to_string(path) else {
        return AppPreferences::default();
    };
    parse_preferences(&raw)
}

pub(super) fn save_preferences(preferences: &AppPreferences) -> Result<()> {
    let Some(path) = preferences_file_path() else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serialize_preferences(preferences))?;
    Ok(())
}

pub(super) fn ensure_preferences_file(preferences: &AppPreferences) -> Result<std::path::PathBuf> {
    let Some(path) = preferences_file_path() else {
        anyhow::bail!("could not resolve application support directory");
    };
    if !path.exists() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, serialize_preferences(preferences))?;
    }
    Ok(path)
}

fn parse_preferences(raw: &str) -> AppPreferences {
    let mut preferences = AppPreferences::default();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "syntax_theme" => {
                if let Some(theme) = parse_theme(value) {
                    preferences.syntax_theme = theme;
                }
            }
            "sidebar_visible" => update_bool(value, &mut preferences.sidebar_visible),
            "status_bar_pinned" => update_bool(value, &mut preferences.status_bar_pinned),
            "focus_mode" => update_bool(value, &mut preferences.focus_mode),
            "typewriter_mode" => update_bool(value, &mut preferences.typewriter_mode),
            "focus_highlight_mode" => update_bool(value, &mut preferences.focus_highlight_mode),
            "font_size" => {
                if let Ok(size) = value.parse::<u16>() {
                    preferences.font_size = size.clamp(12, 28);
                }
            }
            _ => {}
        }
    }
    preferences
}

fn serialize_preferences(preferences: &AppPreferences) -> String {
    format!(
        "syntax_theme={}\nsidebar_visible={}\nstatus_bar_pinned={}\nfocus_mode={}\ntypewriter_mode={}\nfocus_highlight_mode={}\nfont_size={}\n",
        theme_key(preferences.syntax_theme),
        preferences.sidebar_visible,
        preferences.status_bar_pinned,
        preferences.focus_mode,
        preferences.typewriter_mode,
        preferences.focus_highlight_mode,
        preferences.font_size,
    )
}

fn parse_theme(value: &str) -> Option<SyntaxTheme> {
    match value.trim().to_ascii_lowercase().as_str() {
        "default" => Some(SyntaxTheme::Default),
        "dracula" => Some(SyntaxTheme::Dracula),
        "solarized" => Some(SyntaxTheme::Solarized),
        "github" => Some(SyntaxTheme::GitHub),
        _ => None,
    }
}

fn theme_key(theme: SyntaxTheme) -> &'static str {
    match theme {
        SyntaxTheme::Default => "default",
        SyntaxTheme::Dracula => "dracula",
        SyntaxTheme::Solarized => "solarized",
        SyntaxTheme::GitHub => "github",
    }
}

fn update_bool(value: &str, target: &mut bool) {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => *target = true,
        "false" | "0" | "no" | "off" => *target = false,
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_preferences_and_ignores_unknown_keys() {
        let preferences = parse_preferences(
            "syntax_theme=github\nsidebar_visible=false\nstatus_bar_pinned=yes\nfocus_mode=off\ntypewriter_mode=1\nfocus_highlight_mode=true\nfont_size=99\nunknown=value\n",
        );
        assert_eq!(preferences.syntax_theme, SyntaxTheme::GitHub);
        assert!(!preferences.sidebar_visible);
        assert!(preferences.status_bar_pinned);
        assert!(!preferences.focus_mode);
        assert!(preferences.typewriter_mode);
        assert!(preferences.focus_highlight_mode);
        assert_eq!(preferences.font_size, 28);
    }

    #[test]
    fn serializes_preferences_as_stable_key_value_lines() {
        let preferences = AppPreferences {
            syntax_theme: SyntaxTheme::Dracula,
            sidebar_visible: false,
            status_bar_pinned: true,
            focus_mode: false,
            typewriter_mode: true,
            focus_highlight_mode: false,
            font_size: 18,
        };
        let raw = serialize_preferences(&preferences);
        assert!(raw.contains("syntax_theme=dracula\n"));
        assert!(raw.contains("sidebar_visible=false\n"));
        assert!(raw.contains("typewriter_mode=true\n"));
    }
}
