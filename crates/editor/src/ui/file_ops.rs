use std::{
    ops::Range,
    path::{Component, Path, PathBuf},
    time::SystemTime,
};

use anyhow::Result;
use gpui::{Context, Window};
use gpui_component::input::{Copy, Cut, Paste, SelectAll};

use crate::{
    EditCommand, EditorViewMode,
    core::controller::{EditorEffects, FileSyncEvent},
};

use super::view::MarkdownEditor;

pub(super) const DEFAULT_IMAGE_ASSET_DIR: &str = "assets";

impl MarkdownEditor {
    pub fn set_image_asset_dir(&mut self, asset_dir: impl Into<PathBuf>) {
        self.image_asset_dir = normalize_image_asset_dir(asset_dir.into());
    }

    pub fn image_asset_dir(&self) -> &Path {
        &self.image_asset_dir
    }

    pub fn cut_selection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.dispatch_action(Box::new(Cut), cx);
    }

    pub fn copy_selection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.dispatch_action(Box::new(Copy), cx);
    }

    pub fn paste_at_cursor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(item) = cx.read_from_clipboard() {
            for entry in item.entries() {
                if let gpui::ClipboardEntry::Image(image) = entry {
                    if let Some(path) = self.save_clipboard_image(image, cx) {
                        let markdown = self.pasted_image_markdown_for_path(&path);
                        let effects = self
                            .controller
                            .dispatch(EditCommand::ReplaceSelection { text: markdown });
                        if effects.changed {
                            self.schedule_autosave(window, cx);
                        }
                        self.apply_effects(window, cx, effects);
                        return;
                    }
                }
            }
        }
        window.dispatch_action(Box::new(Paste), cx);
    }

    fn save_clipboard_image(
        &self,
        image: &gpui::Image,
        _cx: &mut Context<Self>,
    ) -> Option<PathBuf> {
        let assets_dir = self.ensure_image_asset_dir()?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();

        let format = image.format();
        let ext = match format {
            gpui::ImageFormat::Png => "png",
            gpui::ImageFormat::Jpeg => "jpg",
            gpui::ImageFormat::Gif => "gif",
            gpui::ImageFormat::Webp => "webp",
            gpui::ImageFormat::Bmp => "bmp",
            gpui::ImageFormat::Tiff => "tiff",
            gpui::ImageFormat::Svg => "svg",
        };

        let path = unique_image_asset_path(&assets_dir, &format!("paste-{timestamp}"), ext);
        std::fs::write(&path, image.bytes()).ok()?;
        Some(path)
    }

    pub(crate) fn copy_image_into_assets(&self, image_path: &Path) -> Option<PathBuf> {
        let assets_dir = self.ensure_image_asset_dir()?;
        if path_is_inside_dir(image_path, &assets_dir) {
            return Some(image_path.to_path_buf());
        }

        let ext = image_asset_extension(image_path)?;
        let stem = image_path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("image");
        let target = unique_image_asset_path(&assets_dir, stem, &ext);
        std::fs::copy(image_path, &target).ok()?;
        Some(target)
    }

    fn ensure_image_asset_dir(&self) -> Option<PathBuf> {
        let doc_dir = self.controller.current_document_dir()?;
        let assets_dir = doc_dir.join(&self.image_asset_dir);
        std::fs::create_dir_all(&assets_dir).ok()?;
        Some(assets_dir)
    }

    pub fn select_all(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.dispatch_action(Box::new(SelectAll), cx);
    }
    pub fn set_view_mode(
        &mut self,
        view_mode: EditorViewMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let effects = self.controller.set_view_mode(view_mode);
        self.apply_effects(window, cx, effects);
        self.focus_input(window, cx);
    }

    pub fn toggle_view_mode(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let effects = self.controller.toggle_view_mode();
        self.apply_effects(window, cx, effects);
        self.focus_input(window, cx);
    }

    pub fn select_block_start(
        &mut self,
        block_id: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let effects = self.controller.select_block_start(block_id);
        self.apply_effects(window, cx, effects);
        self.focus_input(window, cx);
    }

    pub fn select_source_offset(
        &mut self,
        byte_offset: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let effects = self.controller.select_source_offset(byte_offset);
        self.apply_effects(window, cx, effects);
        self.focus_input(window, cx);
    }

    pub fn replace_source_range(
        &mut self,
        range: Range<usize>,
        replacement: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let effects = self.controller.replace_source_range(range, replacement);
        self.apply_effects(window, cx, effects);
    }

    pub fn current_document_dir(&self) -> Option<PathBuf> {
        self.controller.current_document_dir()
    }

    pub fn document_path(&self) -> Option<&PathBuf> {
        self.controller.document_path()
    }

    pub fn open_path(
        &mut self,
        path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        let effects = self.controller.open_path(path)?;
        self.apply_effects(window, cx, effects);
        Ok(())
    }

    pub fn new_untitled(
        &mut self,
        suggested_path: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let effects = self.controller.new_untitled(suggested_path);
        self.apply_effects(window, cx, effects);
    }

    pub fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Result<()> {
        let effects = self.controller.save()?;
        self.apply_effects(window, cx, effects);
        Ok(())
    }

    pub fn save_as(
        &mut self,
        path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        let effects = self.controller.save_as(path)?;
        self.apply_effects(window, cx, effects);
        Ok(())
    }

    pub fn apply_file_event(
        &mut self,
        event: FileSyncEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<PathBuf> {
        let effects = self.controller.apply_file_event(event);
        let reload_path = effects.reload_path.clone();
        self.apply_effects(window, cx, effects);
        reload_path
    }

    pub fn apply_file_event_without_window(
        &mut self,
        event: FileSyncEvent,
        cx: &mut Context<Self>,
    ) -> Option<PathBuf> {
        let effects = self.controller.apply_file_event(event);
        let reload_path = effects.reload_path.clone();
        self.snapshot = self.controller.snapshot();
        if effects.changed || effects.selection_changed {
            self.emit_changed(cx);
        }
        reload_path
    }

    pub fn apply_disk_state(
        &mut self,
        path: PathBuf,
        disk_text: String,
        modified_at: Option<SystemTime>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let effects = self
            .controller
            .apply_disk_state(path, disk_text, modified_at);
        self.apply_effects(window, cx, effects);
    }

    pub fn reload_conflict(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let effects = self.controller.dispatch(EditCommand::ReloadConflict);
        self.apply_effects(window, cx, effects);
        self.focus_input(window, cx);
    }

    pub fn keep_current_conflict(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let effects = self.controller.dispatch(EditCommand::KeepCurrentConflict);
        self.apply_effects(window, cx, effects);
        self.focus_input(window, cx);
    }

    pub(super) fn toggle_task_marker(
        &mut self,
        range: Range<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let effects = self.controller.toggle_task_range(range);
        if effects.changed {
            self.schedule_autosave(window, cx);
        }
        self.apply_effects(window, cx, effects);
        self.focus_input(window, cx);
    }

    pub(super) fn insert_table_row(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let effects = self.controller.insert_table_row();
        if effects.changed {
            self.schedule_autosave(window, cx);
        }
        self.apply_effects(window, cx, effects);
        self.focus_input(window, cx);
    }

    pub(super) fn delete_table_row(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let effects = self.controller.delete_table_row();
        if effects.changed {
            self.schedule_autosave(window, cx);
        }
        self.apply_effects(window, cx, effects);
        self.focus_input(window, cx);
    }

    pub(super) fn insert_table_column(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let effects = self.controller.insert_table_column();
        if effects.changed {
            self.schedule_autosave(window, cx);
        }
        self.apply_effects(window, cx, effects);
        self.focus_input(window, cx);
    }

    pub(super) fn delete_table_column(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let effects = self.controller.delete_table_column();
        if effects.changed {
            self.schedule_autosave(window, cx);
        }
        self.apply_effects(window, cx, effects);
        self.focus_input(window, cx);
    }

    pub(super) fn align_table_column(
        &mut self,
        alignment: crate::core::table::TableColumnAlignment,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let effects = self.controller.align_table_column(alignment);
        if effects.changed {
            self.schedule_autosave(window, cx);
        }
        self.apply_effects(window, cx, effects);
        self.focus_input(window, cx);
    }

    pub(super) fn apply_effects(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        effects: EditorEffects,
    ) {
        self.snapshot = self.controller.snapshot();
        self.sync_input_from_snapshot(window, cx);
        if effects.changed || effects.selection_changed {
            if effects.selection_changed {
                self.reset_cursor_blink(window, cx);
            }
            self.check_slash_command();
            self.check_math_completion();
            self.emit_changed(cx);
        }
        if effects.selection_changed {
            self.scroll_cursor_into_view(window, cx);
        }
    }
}

fn normalize_image_asset_dir(path: PathBuf) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => normalized.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::Prefix(_) | Component::RootDir => {
                return PathBuf::from(DEFAULT_IMAGE_ASSET_DIR);
            }
        }
    }

    if normalized.as_os_str().is_empty() {
        PathBuf::from(DEFAULT_IMAGE_ASSET_DIR)
    } else {
        normalized
    }
}

fn image_asset_extension(path: &Path) -> Option<String> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    matches!(
        ext.as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" | "ico" | "tiff" | "tif"
    )
    .then_some(ext)
}

fn unique_image_asset_path(asset_dir: &Path, stem: &str, ext: &str) -> PathBuf {
    let stem = sanitize_asset_file_stem(stem);
    let ext = sanitize_asset_extension(ext);
    let first = asset_dir.join(format!("{stem}.{ext}"));
    if !first.exists() {
        return first;
    }

    for index in 2.. {
        let candidate = asset_dir.join(format!("{stem}-{index}.{ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }

    unreachable!("unbounded suffix search should always find an available asset path")
}

fn sanitize_asset_file_stem(stem: &str) -> String {
    let sanitized = stem
        .chars()
        .map(|ch| {
            if ch.is_control() || matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
            {
                '-'
            } else {
                ch
            }
        })
        .collect::<String>()
        .trim_matches([' ', '.'])
        .to_string();

    if sanitized.is_empty() {
        "image".to_string()
    } else {
        sanitized
    }
}

fn sanitize_asset_extension(ext: &str) -> String {
    let sanitized = ext
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();

    if sanitized.is_empty() {
        "png".to_string()
    } else {
        sanitized
    }
}

fn path_is_inside_dir(path: &Path, dir: &Path) -> bool {
    let Ok(path) = path.canonicalize() else {
        return false;
    };
    let Ok(dir) = dir.canonicalize() else {
        return false;
    };
    path.starts_with(dir)
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn image_asset_dir_normalization_accepts_relative_dirs() {
        assert_eq!(
            normalize_image_asset_dir(PathBuf::from("./media/images")),
            PathBuf::from("media/images")
        );
        assert_eq!(
            normalize_image_asset_dir(PathBuf::from("my images")),
            PathBuf::from("my images")
        );
    }

    #[test]
    fn image_asset_dir_normalization_rejects_unsafe_dirs() {
        assert_eq!(
            normalize_image_asset_dir(PathBuf::from("../outside")),
            PathBuf::from(DEFAULT_IMAGE_ASSET_DIR)
        );
        assert_eq!(
            normalize_image_asset_dir(PathBuf::from("/tmp/assets")),
            PathBuf::from(DEFAULT_IMAGE_ASSET_DIR)
        );
        assert_eq!(
            normalize_image_asset_dir(PathBuf::new()),
            PathBuf::from(DEFAULT_IMAGE_ASSET_DIR)
        );
    }

    #[test]
    fn unique_image_asset_path_preserves_names_and_avoids_collisions() {
        let test_root = std::env::temp_dir().join(format!(
            "vellum-image-assets-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::create_dir_all(&test_root).unwrap();

        let first = unique_image_asset_path(&test_root, "diagram", "PNG");
        assert_eq!(first, test_root.join("diagram.png"));
        std::fs::write(&first, b"one").unwrap();

        let second = unique_image_asset_path(&test_root, "diagram", "PNG");
        assert_eq!(second, test_root.join("diagram-2.png"));

        std::fs::remove_file(first).unwrap();
        std::fs::remove_dir(test_root).unwrap();
    }

    #[test]
    fn asset_file_stems_are_sanitized_without_losing_unicode() {
        assert_eq!(sanitize_asset_file_stem("图 1: draft"), "图 1- draft");
        assert_eq!(sanitize_asset_file_stem(".."), "image");
        assert_eq!(sanitize_asset_file_stem("a/b\\c"), "a-b-c");
    }
}
