use std::{fs, path::Component};

use anyhow::Result;
use editor::{DEFAULT_BODY_FONT_SIZE, EditorViewMode, SyntaxTheme, normalize_body_font_size};

use crate::path::preferences_file_path;

#[derive(Debug, Clone)]
pub(super) struct AppPreferences {
    pub syntax_theme: SyntaxTheme,
    pub view_mode: EditorViewMode,
    pub sidebar_visible: bool,
    pub status_bar_pinned: bool,
    pub focus_mode: bool,
    pub typewriter_mode: bool,
    pub focus_highlight_mode: bool,
    pub image_asset_dir: String,
    pub font_size: u16,
}

impl Default for AppPreferences {
    fn default() -> Self {
        Self {
            syntax_theme: SyntaxTheme::Default,
            view_mode: EditorViewMode::LivePreview,
            sidebar_visible: true,
            status_bar_pinned: false,
            focus_mode: false,
            typewriter_mode: false,
            focus_highlight_mode: false,
            image_asset_dir: "assets".to_string(),
            font_size: DEFAULT_BODY_FONT_SIZE,
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
            "view_mode" => {
                if let Some(view_mode) = parse_view_mode(value) {
                    preferences.view_mode = view_mode;
                }
            }
            "sidebar_visible" => update_bool(value, &mut preferences.sidebar_visible),
            "status_bar_pinned" => update_bool(value, &mut preferences.status_bar_pinned),
            "focus_mode" => update_bool(value, &mut preferences.focus_mode),
            "typewriter_mode" => update_bool(value, &mut preferences.typewriter_mode),
            "focus_highlight_mode" => update_bool(value, &mut preferences.focus_highlight_mode),
            "image_asset_dir" => preferences.image_asset_dir = normalize_image_asset_dir(value),
            "font_size" => {
                if let Ok(size) = value.parse::<u16>() {
                    preferences.font_size = normalize_body_font_size(size);
                }
            }
            _ => {}
        }
    }
    preferences
}

fn serialize_preferences(preferences: &AppPreferences) -> String {
    format!(
        "syntax_theme={}\nview_mode={}\nsidebar_visible={}\nstatus_bar_pinned={}\nfocus_mode={}\ntypewriter_mode={}\nfocus_highlight_mode={}\nimage_asset_dir={}\nfont_size={}\n",
        theme_key(preferences.syntax_theme),
        view_mode_key(preferences.view_mode),
        preferences.sidebar_visible,
        preferences.status_bar_pinned,
        preferences.focus_mode,
        preferences.typewriter_mode,
        preferences.focus_highlight_mode,
        preferences.image_asset_dir,
        preferences.font_size,
    )
}

fn parse_view_mode(value: &str) -> Option<EditorViewMode> {
    match value.trim().to_ascii_lowercase().as_str() {
        "live_preview" | "live-preview" | "livepreview" | "preview" => {
            Some(EditorViewMode::LivePreview)
        }
        "source" | "source_mode" | "source-mode" => Some(EditorViewMode::Source),
        _ => None,
    }
}

fn view_mode_key(view_mode: EditorViewMode) -> &'static str {
    match view_mode {
        EditorViewMode::LivePreview => "live_preview",
        EditorViewMode::Source => "source",
    }
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

pub(super) fn normalize_image_asset_dir(value: &str) -> String {
    parse_image_asset_dir(value).unwrap_or_else(|| AppPreferences::default().image_asset_dir)
}

fn parse_image_asset_dir(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_matches(['"', '\'']);
    if trimmed.is_empty() {
        return None;
    }

    let mut parts = Vec::new();
    for component in std::path::Path::new(trimmed).components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().to_string()),
            Component::CurDir => {}
            Component::ParentDir | Component::Prefix(_) | Component::RootDir => return None,
        }
    }

    (!parts.is_empty()).then(|| parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_preferences_and_ignores_unknown_keys() {
        let preferences = parse_preferences(
            "syntax_theme=github\nview_mode=source\nsidebar_visible=false\nstatus_bar_pinned=yes\nfocus_mode=off\ntypewriter_mode=1\nfocus_highlight_mode=true\nimage_asset_dir=media/images\nfont_size=99\nunknown=value\n",
        );
        assert_eq!(preferences.syntax_theme, SyntaxTheme::GitHub);
        assert_eq!(preferences.view_mode, EditorViewMode::Source);
        assert!(!preferences.sidebar_visible);
        assert!(preferences.status_bar_pinned);
        assert!(!preferences.focus_mode);
        assert!(preferences.typewriter_mode);
        assert!(preferences.focus_highlight_mode);
        assert_eq!(preferences.image_asset_dir, "media/images");
        assert_eq!(preferences.font_size, 28);
    }

    #[test]
    fn serializes_preferences_as_stable_key_value_lines() {
        let preferences = AppPreferences {
            syntax_theme: SyntaxTheme::Dracula,
            view_mode: EditorViewMode::Source,
            sidebar_visible: false,
            status_bar_pinned: true,
            focus_mode: false,
            typewriter_mode: true,
            focus_highlight_mode: false,
            image_asset_dir: "media".to_string(),
            font_size: 18,
        };
        let raw = serialize_preferences(&preferences);
        assert!(raw.contains("syntax_theme=dracula\n"));
        assert!(raw.contains("view_mode=source\n"));
        assert!(raw.contains("sidebar_visible=false\n"));
        assert!(raw.contains("typewriter_mode=true\n"));
        assert!(raw.contains("image_asset_dir=media\n"));
        assert!(raw.contains("font_size=18\n"));
    }

    #[test]
    fn image_asset_dir_pref_rejects_unsafe_paths() {
        let preferences = parse_preferences("image_asset_dir=../outside\n");
        assert_eq!(preferences.image_asset_dir, "assets");

        let preferences = parse_preferences("image_asset_dir=\"./media/images\"\n");
        assert_eq!(preferences.image_asset_dir, "media/images");
    }

    #[test]
    fn normalizes_image_asset_dir_for_ui_input() {
        assert_eq!(normalize_image_asset_dir("media/images"), "media/images");
        assert_eq!(normalize_image_asset_dir("/tmp/assets"), "assets");
        assert_eq!(normalize_image_asset_dir(""), "assets");
    }

    #[test]
    fn parses_view_mode_aliases() {
        assert_eq!(parse_view_mode("live-preview"), Some(EditorViewMode::LivePreview));
        assert_eq!(parse_view_mode("preview"), Some(EditorViewMode::LivePreview));
        assert_eq!(parse_view_mode("source-mode"), Some(EditorViewMode::Source));
        assert_eq!(parse_view_mode("unknown"), None);
    }
}
