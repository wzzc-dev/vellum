use std::fs;

use super::layout::next_untitled_path;
use super::*;
use crate::path::{clear_last_opened_path, read_last_opened_path, write_last_opened_path};

impl VellumApp {
    pub(super) fn refresh_tree(&mut self, cx: &mut Context<Self>) {
        let items = match self.workspace.tree_items() {
            Ok(items) => items,
            Err(err) => {
                self.set_status(format!("Failed to build tree: {err}"));
                Vec::new()
            }
        };

        self.tree_state.update(cx, |state, cx| {
            state.set_items(items, cx);
        });
        cx.notify();
    }

    pub(super) fn set_workspace_root(&mut self, root: Option<PathBuf>, cx: &mut Context<Self>) {
        self.app_state.workspace_root = root.clone();
        match self.workspace.set_root(root) {
            Ok(()) => self.refresh_tree(cx),
            Err(err) => {
                self.set_status(format!("Failed to watch workspace: {err}"));
                cx.notify();
            }
        }
    }

    pub(super) fn open_folder_dialog(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let Some(folder) = FileDialog::new().pick_folder() else {
            return;
        };

        self.set_workspace_root(Some(folder.clone()), cx);
        self.workspace.selected_file = self
            .editor_snapshot
            .path
            .as_ref()
            .filter(|path| path.starts_with(&folder))
            .cloned();
        self.refresh_tree(cx);
        self.set_status(format!("Opened folder {}", folder.display()));
        cx.notify();
    }

    pub(super) fn open_file_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut dialog = FileDialog::new();
        if let Some(dir) = self.current_document_dir() {
            dialog = dialog.set_directory(dir);
        }

        let Some(path) = dialog
            .add_filter("Markdown", &["md", "markdown", "mdown"])
            .pick_file()
        else {
            return;
        };

        self.open_file(path, window, cx);
    }

    pub(super) fn open_file(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        if !is_markdown_path(&path) {
            self.set_status(format!("Ignored non-Markdown file {}", path.display()));
            cx.notify();
            return;
        }

        if self.editor_snapshot.dirty {
            if let Err(err) = self.save_document(window, cx) {
                self.set_status(format!("Save failed before open: {err}"));
                cx.notify();
                return;
            }
            if self.editor_snapshot.dirty {
                return;
            }
        }

        let open_result = self
            .editor
            .update(cx, |editor, cx| editor.open_path(path.clone(), window, cx));
        match open_result {
            Ok(()) => {
                if let Some(root) = path.parent().map(|parent| parent.to_path_buf()) {
                    if self.app_state.workspace_root.as_ref() != Some(&root) {
                        self.set_workspace_root(Some(root), cx);
                    }
                }
                self.workspace.selected_file = Some(path.clone());
                let _ = write_last_opened_path(&path);
                self.clear_status();
                cx.notify();
            }
            Err(err) => {
                self.set_status(format!("Failed to open {}: {err}", path.display()));
                cx.notify();
            }
        }
    }

    pub(super) fn create_new_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let suggested_path = self
            .app_state
            .workspace_root
            .as_ref()
            .map(|root| next_untitled_path(root));

        self.editor.update(cx, |editor, cx| {
            editor.new_untitled(suggested_path.clone(), window, cx);
        });
        self.workspace.selected_file = suggested_path;
        self.clear_status();
        cx.notify();
    }

    pub(super) fn save_document(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        if self.editor_snapshot.path.is_none() && self.app_state.workspace_root.is_none() {
            return self.save_document_as(window, cx);
        }

        if let Err(err) = self.editor.update(cx, |editor, cx| editor.save(window, cx)) {
            if err
                .to_string()
                .contains("cannot save without a target path")
            {
                return self.save_document_as(window, cx);
            }
            return Err(err);
        }

        let saved_path = self.editor.read(cx).document_path().cloned();
        if let Some(path) = saved_path {
            self.workspace.selected_file = Some(path.clone());
            let _ = write_last_opened_path(&path);
        }
        if self.workspace.root.is_some() {
            self.refresh_tree(cx);
        }
        self.clear_status();
        cx.notify();
        Ok(())
    }

    pub(super) fn save_document_as(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        let mut dialog = FileDialog::new().add_filter("Markdown", &["md", "markdown", "mdown"]);
        if let Some(dir) = self.current_document_dir() {
            dialog = dialog.set_directory(dir);
        }
        dialog = dialog.set_file_name(&self.editor_snapshot.display_name);

        let Some(path) = dialog.save_file() else {
            return Ok(());
        };

        self.editor
            .update(cx, |editor, cx| editor.save_as(path.clone(), window, cx))?;

        if let Some(parent) = path.parent().map(|parent| parent.to_path_buf()) {
            if self.app_state.workspace_root.as_ref() != Some(&parent) {
                self.set_workspace_root(Some(parent), cx);
            }
        }

        self.workspace.selected_file = Some(path.clone());
        let _ = write_last_opened_path(&path);
        if self.workspace.root.is_some() {
            self.refresh_tree(cx);
        }
        self.clear_status();
        cx.notify();
        Ok(())
    }

    pub(super) fn restore_last_opened_document(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(path) = read_last_opened_path() else {
            return;
        };

        if !path.exists() {
            clear_last_opened_path();
            self.set_status(format!("Last file unavailable: {}", path.display()));
            cx.notify();
            return;
        }

        if !is_markdown_path(&path) {
            clear_last_opened_path();
            self.set_status(format!("Last file is not Markdown: {}", path.display()));
            cx.notify();
            return;
        }

        self.open_file(path, window, cx);
    }

    pub(super) fn remember_last_opened_document(&self) {
        if let Some(path) = self
            .editor_snapshot
            .path
            .as_ref()
            .filter(|path| is_markdown_path(path))
        {
            let _ = write_last_opened_path(path);
        }
    }

    pub(super) fn poll_workspace(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let events = self.workspace.poll_events();
        if events.is_empty() {
            return;
        }

        let mut should_refresh_tree = false;

        for event in events {
            match event {
                WorkspaceEvent::Changed(path) => {
                    should_refresh_tree = true;
                    if !path.is_file() || !is_markdown_path(&path) {
                        continue;
                    }

                    let Ok(disk_text) = fs::read_to_string(&path) else {
                        continue;
                    };
                    let modified_at = fs::metadata(&path)
                        .ok()
                        .and_then(|meta| meta.modified().ok());
                    self.editor.update(cx, |editor, cx| {
                        editor.handle_disk_change(
                            path.clone(),
                            disk_text.clone(),
                            modified_at,
                            window,
                            cx,
                        );
                    });
                }
                WorkspaceEvent::Removed(path) => {
                    should_refresh_tree = true;
                    if self.workspace.selected_file.as_ref() == Some(&path) {
                        self.workspace.selected_file = None;
                    }
                    self.editor.update(cx, |editor, cx| {
                        editor.handle_disk_removed(path.clone(), window, cx);
                    });
                }
                WorkspaceEvent::Unknown => {
                    should_refresh_tree = true;
                }
            }
        }

        if should_refresh_tree {
            self.refresh_tree(cx);
        }
    }
}
