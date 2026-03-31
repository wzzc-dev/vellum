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
    }

    pub(super) fn set_workspace_root(&mut self, root: Option<PathBuf>, cx: &mut Context<Self>) {
        self.app_state.workspace_root = root.clone();
        match self.workspace.set_root(root) {
            Ok(()) => self.refresh_tree(cx),
            Err(err) => self.set_status(format!("Failed to watch workspace: {err}")),
        }
    }

    pub(super) fn replace_document(
        &mut self,
        document: DocumentState,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.clear_session();
        self.document = document;
        self.remember_last_opened_document();
        window.set_window_title(&self.window_title());
        cx.notify();
    }

    pub(super) fn open_folder_dialog(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let Some(folder) = FileDialog::new().pick_folder() else {
            return;
        };

        self.set_workspace_root(Some(folder.clone()), cx);
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
            return;
        }

        if let Err(err) = self.flush_active_session(true, window, cx) {
            self.set_status(format!("Failed to flush before open: {err}"));
        }

        if self.document.dirty {
            let _ = self.save_document(window, cx);
        }

        match DocumentState::from_disk(path.clone()) {
            Ok(document) => {
                if let Some(root) = path.parent().map(Path::to_path_buf) {
                    if self.app_state.workspace_root.as_ref() != Some(&root) {
                        self.set_workspace_root(Some(root), cx);
                    }
                }

                self.workspace.selected_file = Some(path.clone());
                self.replace_document(document, window, cx);
                self.set_status(format!("Opened {}", path.display()));
            }
            Err(err) => self.set_status(format!("Failed to open {}: {err}", path.display())),
        }
    }

    pub(super) fn create_new_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Err(err) = self.flush_active_session(true, window, cx) {
            self.set_status(format!("Failed to flush before new file: {err}"));
        }

        let suggested_path = self
            .app_state
            .workspace_root
            .as_ref()
            .map(|root| next_untitled_path(root));
        let document = DocumentState::new_empty(None, suggested_path.clone());

        self.workspace.selected_file = suggested_path;
        self.replace_document(document, window, cx);
        self.set_status("New file");
    }

    pub(super) fn save_document(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        self.flush_active_session(false, window, cx)?;

        if self.document.path.is_none() && self.document.suggested_path().is_none() {
            return self.save_document_as(window, cx);
        }

        self.document.save_now()?;
        if let Some(path) = &self.document.path {
            self.workspace.selected_file = Some(path.clone());
        }
        self.remember_last_opened_document();
        window.set_window_title(&self.window_title());
        self.set_status(format!("Saved {}", self.document.display_name()));
        cx.notify();
        Ok(())
    }

    pub(super) fn save_document_as(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        self.flush_active_session(false, window, cx)?;

        let mut dialog = FileDialog::new().add_filter("Markdown", &["md", "markdown", "mdown"]);
        if let Some(dir) = self.current_document_dir() {
            dialog = dialog.set_directory(dir);
        }
        dialog = dialog.set_file_name(&self.document.display_name());

        let Some(path) = dialog.save_file() else {
            return Ok(());
        };

        if let Some(parent) = path.parent().map(Path::to_path_buf) {
            if self.app_state.workspace_root.as_ref() != Some(&parent) {
                self.set_workspace_root(Some(parent), cx);
            }
        }

        self.document.set_path(path.clone());
        self.document.save_now()?;
        self.workspace.selected_file = Some(path.clone());
        self.remember_last_opened_document();
        window.set_window_title(&self.window_title());
        self.set_status(format!("Saved {}", path.display()));
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
            return;
        }

        if !is_markdown_path(&path) {
            clear_last_opened_path();
            self.set_status(format!("Last file is not Markdown: {}", path.display()));
            return;
        }

        self.open_file(path, window, cx);
    }

    pub(super) fn remember_last_opened_document(&self) {
        if let Some(path) = self
            .document
            .path
            .as_ref()
            .filter(|path| is_markdown_path(path))
        {
            let _ = write_last_opened_path(path);
        }
    }
}
