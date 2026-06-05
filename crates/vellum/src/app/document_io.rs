use std::{fs, path::Path};

use super::layout::next_untitled_path;
use super::*;
use crate::path::{
    add_recent_file, clear_last_opened_path, read_last_opened_path, remove_recent_file,
    write_last_opened_path, write_recent_files,
};
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

        if !path.is_file() {
            self.forget_unavailable_file(&path);
            cx.notify();
            return;
        }

        for (i, tab) in self.tabs.iter().enumerate() {
            if tab.editor.read(cx).document_path() == Some(&path) {
                self.switch_to_tab(i, window, cx);
                return;
            }
        }

        let new_editor = self.new_configured_editor(window, cx);
        let open_result =
            new_editor.update(cx, |editor, cx| editor.open_path(path.clone(), window, cx));
        match open_result {
            Ok(()) => {
                self.tabs.push(EditorTab { editor: new_editor });
                self.active_tab_index = self.tabs.len() - 1;
                self.editor_snapshot = self.active_editor_entity().read(cx).snapshot();
                self.subscribe_active_editor(window, cx);

                if let Some(root) = workspace_root_for_document_path(
                    self.app_state.workspace_root.as_deref(),
                    &path,
                )
                {
                    if self.app_state.workspace_root.as_ref() != Some(&root) {
                        if self.set_workspace_root(Some(root), cx) {
                            self.refresh_tree(cx);
                        }
                    }
                }
                self.workspace.selected_file = Some(path.clone());
                self.remember_document_path(&path);
                self.clear_status();
                cx.notify();
            }
            Err(err) => {
                self.set_status(format!("Failed to open {}: {err}", path.display()));
                cx.notify();
            }
        }
    }

    pub(super) fn open_file_in_current_tab(
        &mut self,
        path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !is_markdown_path(&path) {
            self.set_status(format!("Ignored non-Markdown file {}", path.display()));
            cx.notify();
            return;
        }

        if !path.is_file() {
            self.forget_unavailable_file(&path);
            cx.notify();
            return;
        }

        let open_result = self
            .active_editor_entity()
            .update(cx, |editor, cx| editor.open_path(path.clone(), window, cx));
        match open_result {
            Ok(()) => {
                if let Some(root) = workspace_root_for_document_path(
                    self.app_state.workspace_root.as_deref(),
                    &path,
                )
                {
                    if self.app_state.workspace_root.as_ref() != Some(&root) {
                        if self.set_workspace_root(Some(root), cx) {
                            self.refresh_tree(cx);
                        }
                    }
                }
                self.workspace.selected_file = Some(path.clone());
                self.remember_document_path(&path);
                self.clear_status();
                self.editor_snapshot = self.active_editor_entity().read(cx).snapshot();
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

        let new_editor = self.new_configured_editor(window, cx);
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

        if let Err(err) = self
            .active_editor_entity()
            .update(cx, |editor, cx| editor.save(window, cx))
        {
            if err
                .to_string()
                .contains("cannot save without a target path")
            {
                return self.save_document_as(window, cx);
            }
            return Err(err);
        }

        let saved_path = self
            .active_editor_entity()
            .read(cx)
            .document_path()
            .cloned();
        if let Some(path) = saved_path {
            self.workspace.selected_file = Some(path.clone());
            self.remember_document_path(&path);
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
        if let Some(root) =
            workspace_root_for_document_path(self.app_state.workspace_root.as_deref(), &path)
        {
            if self.app_state.workspace_root.as_ref() != Some(&root) {
                if self.set_workspace_root(Some(root), cx) {
                    self.refresh_tree(cx);
                    refreshed_tree = true;
                }
            }
        }

        self.workspace.selected_file = Some(path.clone());
        self.remember_document_path(&path);
        if self.workspace.root.is_some() && !refreshed_tree {
            self.refresh_tree(cx);
        }
        self.clear_status();
        cx.notify();
        Ok(())
    }

    pub(super) fn export_html_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.export_html_dialog_with_open(window, cx, false);
    }

    pub(super) fn export_print_html_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.export_html_dialog_with_open(window, cx, true);
    }

    fn export_html_dialog_with_open(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        open_after_export: bool,
    ) {
        let current_dir = self.current_document_dir();
        let default_name = self.default_html_export_name();
        let document_text = self.editor_snapshot.document_text.clone();
        let display_name = self.editor_snapshot.display_name.clone();
        let view = cx.entity();

        window
            .spawn(cx, async move |cx| {
                let mut dialog = FileDialog::new()
                    .add_filter("HTML", &["html", "htm"])
                    .set_file_name(&default_name);
                if let Some(dir) = current_dir.as_ref() {
                    dialog = dialog.set_directory(dir);
                }

                let Some(path) = dialog.save_file() else {
                    return;
                };

                let export_result = super::export::export_markdown_to_html_file(
                    &document_text,
                    &display_name,
                    current_dir.as_deref(),
                    &path,
                );

                let _ = cx.update_window_entity(&view, |this, _, cx| {
                    match export_result {
                        Ok(()) if open_after_export => match open_html_with_system(&path) {
                            Ok(()) => this.set_status(format!(
                                "Exported and opened HTML at {}. Use browser Print to save PDF.",
                                path.display()
                            )),
                            Err(err) => this.set_status(format!(
                                "Exported HTML to {} but could not open it: {err}",
                                path.display()
                            )),
                        },
                        Ok(()) => {
                            this.set_status(format!("Exported HTML to {}", path.display()));
                        }
                        Err(err) => {
                            this.set_status(format!("Export failed: {err}"));
                        }
                    }
                    cx.notify();
                });
            })
            .detach();
    }

    pub(super) fn open_preferences_file(&mut self, cx: &mut Context<Self>) {
        match super::preferences::ensure_preferences_file(&self.preferences) {
            Ok(path) => {
                if let Err(err) = open_text_with_system(&path) {
                    self.set_status(format!(
                        "Preferences saved at {} but could not be opened: {err}",
                        path.display()
                    ));
                } else {
                    self.set_status(format!("Opened preferences at {}", path.display()));
                }
            }
            Err(err) => self.set_status(format!("Failed to open preferences: {err}")),
        }
        cx.notify();
    }

    fn default_html_export_name(&self) -> String {
        let mut name = self.editor_snapshot.display_name.clone();
        for suffix in [".markdown", ".mdown", ".md"] {
            if name.to_ascii_lowercase().ends_with(suffix) {
                let keep = name.len() - suffix.len();
                name.truncate(keep);
                break;
            }
        }
        if name.is_empty() {
            name = "Untitled".to_string();
        }
        format!("{name}.html")
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
            let active_path_before = self
                .active_editor()
                .and_then(|editor| editor.read(cx).document_path().cloned());

            match &event {
                WorkspaceEvent::Removed(path) => {
                    if self
                        .workspace
                        .selected_file
                        .as_ref()
                        .is_some_and(|selected_file| path_is_or_descends(selected_file, path))
                    {
                        self.workspace.selected_file = None;
                    }
                }
                WorkspaceEvent::Relocated { from, to } => {
                    if let Some(relocated_path) = self
                        .workspace
                        .selected_file
                        .as_ref()
                        .and_then(|selected_file| relocated_path(selected_file, from, to))
                    {
                        self.workspace.selected_file = Some(relocated_path);
                    }
                    self.persist_relocated_paths(from, to, active_path_before.as_deref());
                }
                WorkspaceEvent::Changed(_) | WorkspaceEvent::Unknown => {}
            }

            let mut reloads = Vec::new();
            for tab in self.tabs.iter() {
                let editor = tab.editor.clone();
                let reload_path = editor.update(cx, |editor, cx| {
                    editor.apply_file_event(map_workspace_event_for_editor(&event), window, cx)
                });
                if let Some(path) = reload_path {
                    reloads.push((editor, path));
                }
            }

            for (editor, path) in reloads {
                if !path.is_file() || !is_markdown_path(&path) {
                    continue;
                }

                let Ok(disk_text) = fs::read_to_string(&path) else {
                    continue;
                };
                let modified_at = fs::metadata(&path)
                    .ok()
                    .and_then(|meta| meta.modified().ok());
                editor.update(cx, |editor, cx| {
                    editor.apply_disk_state(
                        path.clone(),
                        disk_text.clone(),
                        modified_at,
                        window,
                        cx,
                    );
                });
            }
        }

        if should_refresh_tree {
            self.editor_snapshot = self.active_editor_entity().read(cx).snapshot();
            self.refresh_tree(cx);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SystemOpenKind {
    Html,
    Text,
}

fn open_html_with_system(path: &std::path::Path) -> Result<()> {
    system_open_command(path, SystemOpenKind::Html).spawn()?;
    Ok(())
}

fn open_text_with_system(path: &std::path::Path) -> Result<()> {
    system_open_command(path, SystemOpenKind::Text).spawn()?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn system_open_command(path: &std::path::Path, kind: SystemOpenKind) -> std::process::Command {
    let mut command = std::process::Command::new("open");
    if kind == SystemOpenKind::Text {
        command.arg("-t");
    }
    command.arg(path);
    command
}

#[cfg(target_os = "windows")]
fn system_open_command(path: &std::path::Path, kind: SystemOpenKind) -> std::process::Command {
    match kind {
        SystemOpenKind::Html => {
            let mut command = std::process::Command::new("cmd");
            command.arg("/C").arg("start").arg("").arg(path);
            command
        }
        SystemOpenKind::Text => {
            let mut command = std::process::Command::new("notepad");
            command.arg(path);
            command
        }
    }
}

#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
fn system_open_command(path: &std::path::Path, _kind: SystemOpenKind) -> std::process::Command {
    let mut command = std::process::Command::new("xdg-open");
    command.arg(path);
    command
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

fn relocated_path(path: &Path, from: &Path, to: &Path) -> Option<PathBuf> {
    let relative_path = path.strip_prefix(from).ok()?;
    if relative_path.as_os_str().is_empty() {
        Some(to.to_path_buf())
    } else {
        Some(to.join(relative_path))
    }
}

fn path_is_or_descends(path: &Path, root: &Path) -> bool {
    path.starts_with(root)
}

fn relocated_recent_files(files: &[PathBuf], from: &Path, to: &Path) -> Vec<PathBuf> {
    let mut relocated_files = Vec::with_capacity(files.len());
    for path in files {
        let relocated = relocated_path(path, from, to).unwrap_or_else(|| path.clone());
        if !relocated_files.contains(&relocated) {
            relocated_files.push(relocated);
        }
    }
    relocated_files
}

fn workspace_root_for_document_path(current_root: Option<&Path>, path: &Path) -> Option<PathBuf> {
    if let Some(root) = current_root {
        if path.starts_with(root) {
            return Some(root.to_path_buf());
        }
    }

    path.parent().map(Path::to_path_buf)
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
            let _ = std::process::Command::new("pbcopy").arg(&path_str).spawn();
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = std::process::Command::new("wl-copy").arg(&path_str).spawn();
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
            .set_title(if is_dir {
                "Delete Folder"
            } else {
                "Delete File"
            })
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
                    if tab
                        .editor
                        .read(cx)
                        .document_path()
                        .is_some_and(|document_path| {
                            removed_path_matches_document(document_path, &path, is_dir)
                        })
                    {
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
                    let new_editor = self.new_configured_editor(_window, cx);
                    self.tabs.push(EditorTab { editor: new_editor });
                    self.active_tab_index = 0;
                    self.editor_snapshot = self.active_editor_entity().read(cx).snapshot();
                    self.subscribe_active_editor(_window, cx);
                }

                if self
                    .workspace
                    .selected_file
                    .as_ref()
                    .is_some_and(|selected_file| {
                        removed_path_matches_document(selected_file, &path, is_dir)
                    })
                {
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
        let subscription =
            cx.subscribe(
                &input,
                |this: &mut Self, _, event: &InputEvent, cx| match event {
                    InputEvent::PressEnter { .. } => {
                        this.confirm_rename_without_window(cx);
                    }
                    InputEvent::Blur => {
                        this.confirm_rename_without_window(cx);
                    }
                    _ => {}
                },
            );
        self.find_input_subscriptions.push(subscription);
        self.renaming_path = Some(path);
        self.rename_input = Some(input);
        cx.notify();
    }

    pub(super) fn cancel_rename(&mut self, cx: &mut Context<Self>) {
        self.renaming_path = None;
        self.rename_input = None;
        cx.notify();
    }

    pub(super) fn confirm_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(path) = self.renaming_path.take() else {
            return;
        };
        let Some(input) = self.rename_input.take() else {
            self.renaming_path = Some(path);
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

        let active_path_before = self
            .active_editor()
            .and_then(|editor| editor.read(cx).document_path().cloned());

        for tab in self.tabs.iter_mut() {
            let should_relocate = tab
                .editor
                .read(cx)
                .document_path()
                .and_then(|document_path| relocated_path(document_path, &path, &new_path))
                .is_some();
            if should_relocate {
                tab.editor.update(cx, |editor, cx| {
                    editor.apply_file_event(
                        FileSyncEvent::Relocated {
                            from: path.clone(),
                            to: new_path.clone(),
                        },
                        window,
                        cx,
                    );
                });
            }
        }

        if let Some(relocated_selected_file) = self
            .workspace
            .selected_file
            .as_ref()
            .and_then(|selected_file| relocated_path(selected_file, &path, &new_path))
        {
            self.workspace.selected_file = Some(relocated_selected_file);
        }
        self.persist_relocated_paths(&path, &new_path, active_path_before.as_deref());

        self.refresh_tree(cx);
        cx.notify();
    }

    pub(super) fn confirm_rename_without_window(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.renaming_path.take() else {
            return;
        };
        let Some(input) = self.rename_input.take() else {
            self.renaming_path = Some(path);
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

        let active_path_before = self
            .active_editor()
            .and_then(|editor| editor.read(cx).document_path().cloned());

        for tab in self.tabs.iter_mut() {
            let should_relocate = tab
                .editor
                .read(cx)
                .document_path()
                .and_then(|document_path| relocated_path(document_path, &path, &new_path))
                .is_some();
            if should_relocate {
                tab.editor.update(cx, |editor, cx| {
                    editor.apply_file_event_without_window(
                        FileSyncEvent::Relocated {
                            from: path.clone(),
                            to: new_path.clone(),
                        },
                        cx,
                    );
                });
            }
        }

        if let Some(relocated_selected_file) = self
            .workspace
            .selected_file
            .as_ref()
            .and_then(|selected_file| relocated_path(selected_file, &path, &new_path))
        {
            self.workspace.selected_file = Some(relocated_selected_file);
        }
        self.persist_relocated_paths(&path, &new_path, active_path_before.as_deref());

        self.refresh_tree(cx);
        cx.notify();
    }

    fn persist_relocated_paths(&mut self, from: &Path, to: &Path, active_path: Option<&Path>) {
        if let Some(relocated_active_path) =
            active_path.and_then(|path| relocated_path(path, from, to))
        {
            let _ = write_last_opened_path(&relocated_active_path);
        }

        let updated_recent_files = relocated_recent_files(&self.recent_files, from, to);
        if updated_recent_files != self.recent_files {
            self.recent_files = updated_recent_files;
            let _ = write_recent_files(&self.recent_files);
        }
    }

    fn forget_unavailable_file(&mut self, path: &Path) {
        self.recent_files = remove_recent_file(path);
        if read_last_opened_path().as_deref() == Some(path) {
            clear_last_opened_path();
        }
        self.set_status(format!("File unavailable: {}", path.display()));
    }

    fn remember_document_path(&mut self, path: &Path) {
        let _ = write_last_opened_path(path);
        self.recent_files = add_recent_file(path);
    }
}

fn removed_path_matches_document(
    document_path: &Path,
    removed_path: &Path,
    removed_is_dir: bool,
) -> bool {
    if removed_is_dir {
        path_is_or_descends(document_path, removed_path)
    } else {
        document_path == removed_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relocated_path_maps_descendant_path() {
        let from = PathBuf::from("workspace/drafts");
        let to = PathBuf::from("workspace/archive");
        let path = PathBuf::from("workspace/drafts/chapter-one/note.md");

        assert_eq!(
            relocated_path(&path, &from, &to),
            Some(PathBuf::from("workspace/archive/chapter-one/note.md"))
        );
    }

    #[test]
    fn relocated_path_maps_exact_path() {
        let from = PathBuf::from("workspace/drafts/note.md");
        let to = PathBuf::from("workspace/archive/note.md");

        assert_eq!(relocated_path(&from, &from, &to), Some(to));
    }

    #[test]
    fn relocated_path_ignores_sibling_with_shared_prefix() {
        let from = PathBuf::from("workspace/drafts");
        let to = PathBuf::from("workspace/archive");
        let path = PathBuf::from("workspace/drafts-old/note.md");

        assert_eq!(relocated_path(&path, &from, &to), None);
    }

    #[test]
    fn removed_path_matches_descendant_only_for_folders() {
        let root = PathBuf::from("workspace/drafts");
        let path = PathBuf::from("workspace/drafts/chapter-one/note.md");

        assert!(removed_path_matches_document(&path, &root, true));
        assert!(!removed_path_matches_document(&path, &root, false));
    }

    #[test]
    fn relocated_recent_files_updates_descendants_and_deduplicates() {
        let from = PathBuf::from("workspace/drafts");
        let to = PathBuf::from("workspace/archive");
        let files = vec![
            PathBuf::from("workspace/drafts/chapter-one/note.md"),
            PathBuf::from("workspace/archive/chapter-one/note.md"),
            PathBuf::from("workspace/drafts/intro.md"),
            PathBuf::from("workspace/other.md"),
        ];

        assert_eq!(
            relocated_recent_files(&files, &from, &to),
            vec![
                PathBuf::from("workspace/archive/chapter-one/note.md"),
                PathBuf::from("workspace/archive/intro.md"),
                PathBuf::from("workspace/other.md"),
            ]
        );
    }

    #[test]
    fn workspace_root_for_document_path_preserves_existing_containing_root() {
        let root = PathBuf::from("/notes/project");
        let path = PathBuf::from("/notes/project/drafts/chapter.md");

        assert_eq!(
            workspace_root_for_document_path(Some(&root), &path),
            Some(root)
        );
    }

    #[test]
    fn workspace_root_for_document_path_uses_parent_for_external_file() {
        let root = PathBuf::from("/notes/project");
        let path = PathBuf::from("/notes/other/chapter.md");

        assert_eq!(
            workspace_root_for_document_path(Some(&root), &path),
            Some(PathBuf::from("/notes/other"))
        );
    }

    #[test]
    fn workspace_root_for_document_path_does_not_match_sibling_prefix() {
        let root = PathBuf::from("/notes/project");
        let path = PathBuf::from("/notes/project-old/chapter.md");

        assert_eq!(
            workspace_root_for_document_path(Some(&root), &path),
            Some(PathBuf::from("/notes/project-old"))
        );
    }

    #[test]
    fn html_open_uses_system_html_handler() {
        let (program, args) = command_parts(SystemOpenKind::Html, "draft.html");

        #[cfg(target_os = "macos")]
        {
            assert_eq!(program, "open");
            assert_eq!(args, vec!["draft.html"]);
        }

        #[cfg(target_os = "windows")]
        {
            assert_eq!(program, "cmd");
            assert_eq!(args, vec!["/C", "start", "", "draft.html"]);
        }

        #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
        {
            assert_eq!(program, "xdg-open");
            assert_eq!(args, vec!["draft.html"]);
        }
    }

    #[test]
    fn text_open_uses_text_handler() {
        let (program, args) = command_parts(SystemOpenKind::Text, "preferences.conf");

        #[cfg(target_os = "macos")]
        {
            assert_eq!(program, "open");
            assert_eq!(args, vec!["-t", "preferences.conf"]);
        }

        #[cfg(target_os = "windows")]
        {
            assert_eq!(program, "notepad");
            assert_eq!(args, vec!["preferences.conf"]);
        }

        #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
        {
            assert_eq!(program, "xdg-open");
            assert_eq!(args, vec!["preferences.conf"]);
        }
    }

    fn command_parts(kind: SystemOpenKind, path: &str) -> (String, Vec<String>) {
        let command = system_open_command(Path::new(path), kind);
        (
            command.get_program().to_string_lossy().into_owned(),
            command
                .get_args()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect(),
        )
    }
}
