use std::fs;

use super::layout::next_untitled_path;
use super::*;
use crate::path::{clear_last_opened_path, read_last_opened_path, write_last_opened_path};
use editor::FileSyncEvent;

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

    pub(super) fn set_workspace_root(
        &mut self,
        root: Option<PathBuf>,
        cx: &mut Context<Self>,
    ) -> bool {
        self.app_state.workspace_root = root.clone();
        match self.workspace.set_root(root) {
            Ok(()) => true,
            Err(err) => {
                self.set_status(format!("Failed to watch workspace: {err}"));
                cx.notify();
                false
            }
        }
    }

    pub(super) fn request_open_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view = cx.entity();
        window
            .spawn(cx, async move |cx| {
                let folder = FileDialog::new().pick_folder();
                let Some(folder) = folder else {
                    return;
                };

                let _ = cx.update_window_entity(&view, |this, _, cx| {
                    this.apply_open_folder(folder, cx);
                });
            })
            .detach();
    }

    fn apply_open_folder(&mut self, folder: PathBuf, cx: &mut Context<Self>) {
        if !self.set_workspace_root(Some(folder.clone()), cx) {
            return;
        }

        self.workspace.selected_file = self
            .editor_snapshot
            .path
            .as_ref()
            .filter(|path| path.starts_with(&folder))
            .cloned();
        self.set_status(format!("Opened folder {}", folder.display()));
        self.refresh_tree(cx);
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

        for (i, tab) in self.tabs.iter().enumerate() {
            if tab.editor.read(cx).document_path() == Some(&path) {
                self.switch_to_tab(i, window, cx);
                return;
            }
        }

        let new_editor = cx.new(|cx| MarkdownEditor::new(window, cx));
        let open_result = new_editor.update(cx, |editor, cx| editor.open_path(path.clone(), window, cx));
        match open_result {
            Ok(()) => {
                self.tabs.push(EditorTab { editor: new_editor });
                self.active_tab_index = self.tabs.len() - 1;
                self.editor_snapshot = self.active_editor_entity().read(cx).snapshot();
                self.subscribe_active_editor(window, cx);

                if let Some(root) = path.parent().map(|parent| parent.to_path_buf()) {
                    if self.app_state.workspace_root.as_ref() != Some(&root) {
                        if self.set_workspace_root(Some(root), cx) {
                            self.refresh_tree(cx);
                        }
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

        let new_editor = cx.new(|cx| MarkdownEditor::new(window, cx));
        new_editor.update(cx, |editor, cx| {
            editor.new_untitled(suggested_path.clone(), window, cx);
        });

        self.tabs.push(EditorTab { editor: new_editor });
        self.active_tab_index = self.tabs.len() - 1;
        self.editor_snapshot = self.active_editor_entity().read(cx).snapshot();
        self.subscribe_active_editor(window, cx);

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

        if let Err(err) = self.active_editor_entity().update(cx, |editor, cx| editor.save(window, cx)) {
            if err
                .to_string()
                .contains("cannot save without a target path")
            {
                return self.save_document_as(window, cx);
            }
            return Err(err);
        }

        let saved_path = self.active_editor_entity().read(cx).document_path().cloned();
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

        self.active_editor_entity()
            .update(cx, |editor, cx| editor.save_as(path.clone(), window, cx))?;

        let mut refreshed_tree = false;
        if let Some(parent) = path.parent().map(|parent| parent.to_path_buf()) {
            if self.app_state.workspace_root.as_ref() != Some(&parent) {
                if self.set_workspace_root(Some(parent), cx) {
                    self.refresh_tree(cx);
                    refreshed_tree = true;
                }
            }
        }

        self.workspace.selected_file = Some(path.clone());
        let _ = write_last_opened_path(&path);
        if self.workspace.root.is_some() && !refreshed_tree {
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

    pub(super) fn poll_workspace(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let events = self.workspace.poll_events();
        if events.is_empty() {
            return;
        }

        let mut should_refresh_tree = false;

        for event in events {
            should_refresh_tree = true;

            match &event {
                WorkspaceEvent::Removed(path) => {
                    if self.workspace.selected_file.as_ref() == Some(path) {
                        self.workspace.selected_file = None;
                    }
                }
                WorkspaceEvent::Relocated { from, to } => {
                    if self.workspace.selected_file.as_ref() == Some(from) {
                        self.workspace.selected_file = Some(to.clone());
                    }
                }
                WorkspaceEvent::Changed(_) | WorkspaceEvent::Unknown => {}
            }

            let reload_path = self.active_editor_entity().update(cx, |editor, cx| {
                editor.apply_file_event(map_workspace_event_for_editor(&event), window, cx)
            });

            let Some(path) = reload_path else {
                continue;
            };
            if !path.is_file() || !is_markdown_path(&path) {
                continue;
            }

            let Ok(disk_text) = fs::read_to_string(&path) else {
                continue;
            };
            let modified_at = fs::metadata(&path)
                .ok()
                .and_then(|meta| meta.modified().ok());
            self.active_editor_entity().update(cx, |editor, cx| {
                editor.apply_disk_state(path.clone(), disk_text.clone(), modified_at, window, cx);
            });
        }

        if should_refresh_tree {
            self.refresh_tree(cx);
        }
    }
}

fn map_workspace_event_for_editor(event: &WorkspaceEvent) -> FileSyncEvent {
    match event {
        WorkspaceEvent::Changed(path) => FileSyncEvent::Changed(path.clone()),
        WorkspaceEvent::Removed(path) => FileSyncEvent::Removed(path.clone()),
        WorkspaceEvent::Relocated { from, to } => FileSyncEvent::Relocated {
            from: from.clone(),
            to: to.clone(),
        },
        WorkspaceEvent::Unknown => FileSyncEvent::Unknown,
    }
}

impl VellumApp {
    pub(super) fn reveal_in_finder(&self, path: &std::path::Path) {
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open")
                .arg("-R")
                .arg(path)
                .spawn();
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = std::process::Command::new("xdg-open")
                .arg(path.parent().unwrap_or(path))
                .spawn();
        }
    }

    pub(super) fn copy_path_to_clipboard(
        &self,
        path: &std::path::Path,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        let path_str = path.to_string_lossy().to_string();
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("pbcopy")
                .arg(&path_str)
                .spawn();
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = std::process::Command::new("wl-copy")
                .arg(&path_str)
                .spawn();
        }
    }

    pub(super) fn create_new_file_in_folder(
        &mut self,
        folder: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut index = 1;
        let mut new_path = folder.join("Untitled.md");
        while new_path.exists() {
            new_path = folder.join(format!("Untitled {}.md", index));
            index += 1;
        }

        if let Err(err) = fs::write(&new_path, "") {
            self.set_status(format!("Failed to create file: {err}"));
            cx.notify();
            return;
        }

        self.open_file(new_path, window, cx);
        self.refresh_tree(cx);
    }

    pub(super) fn create_new_folder(
        &mut self,
        parent: PathBuf,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut index = 1;
        let mut new_path = parent.join("New Folder");
        while new_path.exists() {
            new_path = parent.join(format!("New Folder {}", index));
            index += 1;
        }

        if let Err(err) = fs::create_dir(&new_path) {
            self.set_status(format!("Failed to create folder: {err}"));
            cx.notify();
            return;
        }

        self.refresh_tree(cx);
    }

    pub(super) fn delete_file(
        &mut self,
        path: PathBuf,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let file_name = path.file_name().unwrap_or_default().to_string_lossy();
        let is_dir = path.is_dir();

        let confirmed = rfd::MessageDialog::new()
            .set_title(if is_dir { "Delete Folder" } else { "Delete File" })
            .set_description(format!(
                "Are you sure you want to delete \"{}\"?",
                file_name
            ))
            .set_buttons(rfd::MessageButtons::YesNo)
            .show();

        if confirmed != rfd::MessageDialogResult::Yes {
            return;
        }

        let result = if is_dir {
            fs::remove_dir_all(&path)
        } else {
            fs::remove_file(&path)
        };

        match result {
            Ok(()) => {
                // Close any tab that has this file open
                let mut indices_to_remove = Vec::new();
                for (i, tab) in self.tabs.iter().enumerate() {
                    if tab.editor.read(cx).document_path() == Some(&path) {
                        indices_to_remove.push(i);
                    }
                }
                // Remove from highest index to lowest to avoid shifting issues
                for i in indices_to_remove.iter().rev() {
                    self.tabs.remove(*i);
                    if self.active_tab_index >= self.tabs.len() {
                        self.active_tab_index = self.tabs.len().saturating_sub(1);
                    } else if self.active_tab_index > *i {
                        self.active_tab_index -= 1;
                    }
                }
                // Ensure at least one tab exists
                if self.tabs.is_empty() {
                    let new_editor = cx.new(|cx| MarkdownEditor::new(_window, cx));
                    self.tabs.push(EditorTab { editor: new_editor });
                    self.active_tab_index = 0;
                    self.editor_snapshot = self.active_editor_entity().read(cx).snapshot();
                    self.subscribe_active_editor(_window, cx);
                }

                if self.workspace.selected_file.as_ref() == Some(&path) {
                    self.workspace.selected_file = None;
                }
                self.refresh_tree(cx);
                cx.notify();
            }
            Err(err) => {
                self.set_status(format!("Failed to delete: {err}"));
                cx.notify();
            }
        }
    }

    pub(super) fn start_rename(
        &mut self,
        path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let file_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let input = cx.new(|cx| {
            let mut state = gpui_component::input::InputState::new(window, cx);
            state.set_value(file_name, window, cx);
            state
        });
        self.renaming_path = Some(path);
        self.rename_input = Some(input);
        cx.notify();
    }

    pub(super) fn cancel_rename(&mut self, cx: &mut Context<Self>) {
        self.renaming_path = None;
        self.rename_input = None;
        cx.notify();
    }

    pub(super) fn confirm_rename(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(path) = self.renaming_path.take() else {
            return;
        };
        let Some(input) = self.rename_input.take() else {
            return;
        };

        let new_name = input.read(cx).value().to_string();
        if new_name.is_empty() {
            cx.notify();
            return;
        }

        let Some(parent) = path.parent() else {
            cx.notify();
            return;
        };

        let new_path = parent.join(&new_name);
        if new_path == path {
            cx.notify();
            return;
        }

        if new_path.exists() {
            self.set_status(format!("A file named \"{}\" already exists", new_name));
            cx.notify();
            return;
        }

        if let Err(err) = fs::rename(&path, &new_path) {
            self.set_status(format!("Failed to rename: {err}"));
            cx.notify();
            return;
        }

        // Update any open tabs that reference this file
        for tab in self.tabs.iter_mut() {
            if tab.editor.read(cx).document_path() == Some(&path) {
                let _ = tab.editor.update(cx, |editor, cx| {
                    editor.open_path(new_path.clone(), window, cx)
                });
            }
        }

        if self.workspace.selected_file.as_ref() == Some(&path) {
            self.workspace.selected_file = Some(new_path.clone());
        }

        self.refresh_tree(cx);
        cx.notify();
    }
}
