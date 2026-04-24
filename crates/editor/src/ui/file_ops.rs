use std::{ops::Range, path::PathBuf, time::SystemTime};

use anyhow::Result;
use gpui::{Context, Window};

use crate::{
    EditCommand, EditorViewMode,
    core::controller::{EditorEffects, FileSyncEvent},
};

use super::view::MarkdownEditor;

impl MarkdownEditor {
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
            self.emit_changed(cx);
        }
    }
}
