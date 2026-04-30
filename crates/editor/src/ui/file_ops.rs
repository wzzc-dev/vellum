use std::{ops::Range, path::PathBuf, time::SystemTime};

use anyhow::Result;
use gpui::{Context, Window};
use gpui_component::input::{Copy, Cut, Paste, SelectAll};

use crate::{
    EditCommand, EditorViewMode,
    core::controller::{EditorEffects, FileSyncEvent},
};

use super::view::MarkdownEditor;

impl MarkdownEditor {
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
                        let relative = self.relative_image_path(&path);
                        let markdown = format!("![]({})", relative);
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

    fn save_clipboard_image(&self, image: &gpui::Image, cx: &mut Context<Self>) -> Option<PathBuf> {
        let doc_dir = self.controller.current_document_dir()?;
        let assets_dir = doc_dir.join("assets");
        std::fs::create_dir_all(&assets_dir).ok()?;

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

        let filename = format!("paste-{}.{}", timestamp, ext);
        let path = assets_dir.join(&filename);
        std::fs::write(&path, image.bytes()).ok()?;
        Some(path)
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
