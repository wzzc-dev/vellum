use std::{path::PathBuf, time::SystemTime};

use anyhow::Result;
use gpui::{Context, Window};

use crate::core::controller::{EditorEffects, FileSyncEvent};

use super::view::MarkdownEditor;

impl MarkdownEditor {
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
