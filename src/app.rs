use std::{
    cmp,
    ops::Range,
    path::{Path, PathBuf},
    rc::Rc,
    time::Duration,
};

use anyhow::Result;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, AppContext, Application, Context, Entity, EntityInputHandler as _,
    InteractiveElement, IntoElement, KeyBinding, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Subscription, Timer, VisualContext, Window, WindowOptions,
    actions, div, px, size,
};
use gpui_component::{
    ActiveTheme, Root, TitleBar,
    button::{Button, ButtonVariants as _},
    input::{Input, InputEvent, InputState},
    list::ListItem,
    resizable::{h_resizable, resizable_panel},
    text::TextView,
    tree::{TreeState, tree},
    v_virtual_list,
};
use rfd::FileDialog;

use crate::{
    editor::{ActiveBlockSession, BlockKind, ConflictState, DocumentState},
    workspace::{WorkspaceEvent, WorkspaceState, is_markdown_path},
};

actions!(
    vellum,
    [
        OpenFile,
        OpenFolder,
        NewFile,
        SaveNow,
        SaveAs,
        ToggleSidebar,
        BoldSelection,
        ItalicSelection,
        LinkSelection,
        PromoteBlock,
        DemoteBlock,
        ExitBlockEdit,
        FocusPrevBlock,
        FocusNextBlock,
    ]
);

const APP_CONTEXT: &str = "VellumApp";
const INPUT_CONTEXT: &str = "Input";
const FLUSH_DELAY: Duration = Duration::from_millis(120);
const AUTOSAVE_DELAY: Duration = Duration::from_millis(700);
const WATCH_POLL_DELAY: Duration = Duration::from_millis(250);
const MAX_EDITOR_WIDTH: f32 = 860.;

pub fn run() -> Result<()> {
    Application::new().run(|cx: &mut App| {
        gpui_component::init(cx);
        bind_keys(cx);

        let options = WindowOptions {
            titlebar: Some(TitleBar::title_bar_options()),
            ..Default::default()
        };

        cx.open_window(options, |window, cx| {
            window.set_window_title("Vellum");
            let view = cx.new(|cx| VellumApp::new(window, cx));
            VellumApp::start_background_tasks(&view, window, cx);
            cx.new(|cx| Root::new(view, window, cx))
        })
        .expect("failed to open main window");

        cx.activate(true);
    });

    Ok(())
}

fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-o", OpenFile, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-shift-o", OpenFolder, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-n", NewFile, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-s", SaveNow, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-shift-s", SaveAs, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-b", BoldSelection, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-i", ItalicSelection, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-k", LinkSelection, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-[", PromoteBlock, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-]", DemoteBlock, Some(APP_CONTEXT)),
        KeyBinding::new("escape", ExitBlockEdit, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-up", FocusPrevBlock, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-down", FocusNextBlock, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-s", SaveNow, Some(INPUT_CONTEXT)),
        KeyBinding::new("ctrl-shift-s", SaveAs, Some(INPUT_CONTEXT)),
        KeyBinding::new("ctrl-b", BoldSelection, Some(INPUT_CONTEXT)),
        KeyBinding::new("ctrl-i", ItalicSelection, Some(INPUT_CONTEXT)),
        KeyBinding::new("ctrl-k", LinkSelection, Some(INPUT_CONTEXT)),
        KeyBinding::new("ctrl-[", PromoteBlock, Some(INPUT_CONTEXT)),
        KeyBinding::new("ctrl-]", DemoteBlock, Some(INPUT_CONTEXT)),
        KeyBinding::new("ctrl-up", FocusPrevBlock, Some(INPUT_CONTEXT)),
        KeyBinding::new("ctrl-down", FocusNextBlock, Some(INPUT_CONTEXT)),
    ]);
}

#[derive(Default)]
struct AppState {
    workspace_root: Option<PathBuf>,
    active_document_id: Option<u64>,
}

struct VellumApp {
    app_state: AppState,
    workspace: WorkspaceState,
    tree_state: Entity<TreeState>,
    document: DocumentState,
    active_session: Option<ActiveBlockSession>,
    input_subscription: Option<Subscription>,
    sidebar_visible: bool,
    status_message: SharedString,
    flush_generation: u64,
    autosave_generation: u64,
}

impl VellumApp {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let tree_state = cx.new(|cx| TreeState::new(cx));

        Self {
            app_state: AppState {
                workspace_root: None,
                active_document_id: Some(1),
            },
            workspace: WorkspaceState::new(),
            tree_state,
            document: DocumentState::new_empty(None, None),
            active_session: None,
            input_subscription: None,
            sidebar_visible: true,
            status_message: SharedString::from("Ready"),
            flush_generation: 0,
            autosave_generation: 0,
        }
    }

    fn start_background_tasks(view: &Entity<Self>, window: &mut Window, cx: &mut App) {
        let view = view.clone();
        window
            .spawn(cx, async move |cx| {
                loop {
                    Timer::after(WATCH_POLL_DELAY).await;
                    if cx
                        .update_window_entity(&view, |this, window, cx| {
                            this.poll_workspace(window, cx);
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .detach();
    }

    fn window_title(&self) -> String {
        let mut title = format!("{} - Vellum", self.document.display_name());
        if self.document.dirty {
            title.push_str(" *");
        }
        title
    }

    fn current_document_dir(&self) -> Option<PathBuf> {
        self.document
            .path
            .as_ref()
            .and_then(|path| path.parent().map(Path::to_path_buf))
            .or_else(|| self.app_state.workspace_root.clone())
    }

    fn set_status(&mut self, status: impl Into<SharedString>) {
        self.status_message = status.into();
    }

    fn clear_session(&mut self) {
        self.input_subscription = None;
        self.active_session = None;
    }

    fn refresh_tree(&mut self, cx: &mut Context<Self>) {
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

    fn set_workspace_root(&mut self, root: Option<PathBuf>, cx: &mut Context<Self>) {
        self.app_state.workspace_root = root.clone();
        match self.workspace.set_root(root) {
            Ok(()) => self.refresh_tree(cx),
            Err(err) => self.set_status(format!("Failed to watch workspace: {err}")),
        }
    }

    fn replace_document(
        &mut self,
        document: DocumentState,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.clear_session();
        self.document = document;
        window.set_window_title(&self.window_title());
        cx.notify();
    }

    fn open_folder_dialog(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let Some(folder) = FileDialog::new().pick_folder() else {
            return;
        };

        self.set_workspace_root(Some(folder.clone()), cx);
        self.set_status(format!("Opened folder {}", folder.display()));
        cx.notify();
    }

    fn open_file_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

    fn open_file(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
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

    fn create_new_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

    fn save_document(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Result<()> {
        self.flush_active_session(false, window, cx)?;

        if self.document.path.is_none() && self.document.suggested_path().is_none() {
            return self.save_document_as(window, cx);
        }

        self.document.save_now()?;
        if let Some(path) = &self.document.path {
            self.workspace.selected_file = Some(path.clone());
        }
        window.set_window_title(&self.window_title());
        self.set_status(format!("Saved {}", self.document.display_name()));
        cx.notify();
        Ok(())
    }

    fn save_document_as(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Result<()> {
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
        window.set_window_title(&self.window_title());
        self.set_status(format!("Saved {}", path.display()));
        cx.notify();
        Ok(())
    }

    fn handle_input_event(
        &mut self,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::Change => {
                if let Some(session) = self.active_session.as_mut() {
                    let block_start = self
                        .document
                        .block_by_id(session.block_id)
                        .map(|block| block.byte_range.start)
                        .unwrap_or(0);

                    let (buffer, cursor_offset) = session
                        .input
                        .update(cx, |input, _| (input.text().to_string(), input.cursor()));
                    session.buffer = buffer;
                    session.cursor_offset = cmp::min(cursor_offset, session.buffer.len());
                    session.anchor_document_offset = block_start + session.cursor_offset;
                }
                self.schedule_flush(window, cx);
                self.schedule_autosave(window, cx);
            }
            InputEvent::Blur => {
                if let Err(err) = self.flush_active_session(true, window, cx) {
                    self.set_status(format!("Failed to flush block: {err}"));
                }
            }
            InputEvent::PressEnter { .. } | InputEvent::Focus => {}
        }
    }

    fn schedule_flush(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.flush_generation = self.flush_generation.wrapping_add(1);
        let token = self.flush_generation;
        let view = cx.entity();
        window
            .spawn(cx, async move |cx| {
                Timer::after(FLUSH_DELAY).await;
                let _ = cx.update_window_entity(&view, |this, window, cx| {
                    if this.flush_generation == token {
                        let _ = this.flush_active_session(false, window, cx);
                    }
                });
            })
            .detach();
    }

    fn schedule_autosave(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.autosave_generation = self.autosave_generation.wrapping_add(1);
        let token = self.autosave_generation;
        let view = cx.entity();
        window
            .spawn(cx, async move |cx| {
                Timer::after(AUTOSAVE_DELAY).await;
                let _ = cx.update_window_entity(&view, |this, window, cx| {
                    if this.autosave_generation == token {
                        let _ = this.flush_active_session(false, window, cx);
                        let _ = this.save_document(window, cx);
                    }
                });
            })
            .detach();
    }

    fn activate_block(&mut self, block_ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        if block_ix >= self.document.blocks.len() {
            return;
        }

        if let Err(err) = self.flush_active_session(true, window, cx) {
            self.set_status(format!("Failed to switch block: {err}"));
        }

        let block = self.document.blocks[block_ix].clone();
        let text = self.document.block_text(&block);
        let view = cx.entity();
        let input = cx.new(|cx| {
            let mut state = match &block.kind {
                BlockKind::CodeFence { language } => InputState::new(window, cx)
                    .code_editor(language.clone().unwrap_or_else(|| "text".to_string()))
                    .line_number(false),
                _ => InputState::new(window, cx)
                    .multi_line(true)
                    .auto_grow(1, 24),
            };
            state.set_value(text.clone(), window, cx);
            state
        });
        let subscription =
            window.subscribe(&input, cx, move |_, event: &InputEvent, window, cx| {
                let _ = view.update(cx, |this, cx| {
                    this.handle_input_event(event, window, cx);
                });
            });

        self.active_session = Some(ActiveBlockSession::new(
            &self.document,
            &block,
            input.clone(),
        ));
        self.input_subscription = Some(subscription);
        input.update(cx, |input, cx| {
            input.focus(window, cx);
            if !text.is_empty() {
                let (row, col) = position_for_byte_offset(&text, activation_cursor_offset(&text));
                input.set_cursor_position(
                    gpui_component::input::Position {
                        line: row as u32,
                        character: col as u32,
                    },
                    window,
                    cx,
                );
            }
        });
        self.set_status(format!("Editing block {}", block_ix + 1));
        cx.notify();
    }

    fn flush_active_session(
        &mut self,
        exit_after_flush: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        let Some(mut session) = self.active_session.take() else {
            return Ok(());
        };

        let block_start = self
            .document
            .block_by_id(session.block_id)
            .map(|block| block.byte_range.start)
            .unwrap_or(0);
        let (buffer, cursor_offset) = session
            .input
            .update(cx, |input, _| (input.text().to_string(), input.cursor()));
        session.buffer = buffer;
        session.cursor_offset = cmp::min(cursor_offset, session.buffer.len());
        session.anchor_document_offset = block_start + session.cursor_offset;

        let Some(block_ix) = self
            .document
            .block_index_by_id(session.block_id)
            .or_else(|| {
                self.document
                    .blocks
                    .iter()
                    .position(|block| session.anchor_document_offset <= block.byte_range.end)
            })
        else {
            self.input_subscription = None;
            return Ok(());
        };

        let old_range = self.document.blocks[block_ix].byte_range.clone();
        let new_anchor = old_range.start + cmp::min(session.cursor_offset, session.buffer.len());
        self.document.replace_range(old_range, &session.buffer);
        window.set_window_title(&self.window_title());

        let new_block_ix = self.document.block_index_at_offset(new_anchor);
        let new_block = self.document.blocks[new_block_ix].clone();
        let new_text = self.document.block_text(&new_block);
        let new_cursor_offset = new_anchor.saturating_sub(new_block.byte_range.start);

        self.input_subscription = None;
        if exit_after_flush {
            self.active_session = None;
            self.set_status("Block synced");
            cx.notify();
            return Ok(());
        }

        session.block_id = new_block.id;
        session.buffer = new_text.clone();
        session.cursor_offset = cmp::min(new_cursor_offset, new_text.len());
        session.anchor_document_offset = new_anchor;

        session.input.update(cx, |input, cx| {
            input.set_value(new_text.clone(), window, cx);
            let (row, col) = position_for_byte_offset(&new_text, session.cursor_offset);
            input.set_cursor_position(
                gpui_component::input::Position {
                    line: row as u32,
                    character: col as u32,
                },
                window,
                cx,
            );
        });

        let view = cx.entity();
        let input = session.input.clone();
        self.input_subscription =
            Some(
                window.subscribe(&input, cx, move |_, event: &InputEvent, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.handle_input_event(event, window, cx);
                    });
                }),
            );
        self.active_session = Some(session);
        self.set_status("Block synced");
        cx.notify();
        Ok(())
    }

    fn current_block_index(&self) -> Option<usize> {
        self.active_session
            .as_ref()
            .and_then(|session| self.document.block_index_by_id(session.block_id))
    }

    fn focus_adjacent_block(
        &mut self,
        direction: isize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.document.blocks.is_empty() {
            return;
        }

        let current = self.current_block_index().unwrap_or(if direction >= 0 {
            0
        } else {
            self.document.blocks.len().saturating_sub(1)
        });
        let next = if direction >= 0 {
            cmp::min(current + 1, self.document.blocks.len().saturating_sub(1))
        } else {
            current.saturating_sub(1)
        };
        self.activate_block(next, window, cx);
    }

    fn exit_edit_mode(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Err(err) = self.flush_active_session(true, window, cx) {
            self.set_status(format!("Failed to exit edit mode: {err}"));
        }
    }

    fn apply_markup(
        &mut self,
        before: &str,
        after: &str,
        placeholder: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(session) = self.active_session.as_ref() else {
            return;
        };

        session.input.update(cx, |input, cx| {
            let selection = input.selected_text_range(true, window, cx);
            let replacement = if let Some(selection) = selection.as_ref() {
                if !selection.range.is_empty() {
                    let mut adjusted = None;
                    let selected = input
                        .text_for_range(selection.range.clone(), &mut adjusted, window, cx)
                        .unwrap_or_default();
                    format!("{before}{selected}{after}")
                } else {
                    format!("{before}{placeholder}{after}")
                }
            } else {
                format!("{before}{placeholder}{after}")
            };

            let range = selection.and_then(|selection| {
                if selection.range.is_empty() {
                    None
                } else {
                    Some(selection.range)
                }
            });
            input.replace_text_in_range(range, &replacement, window, cx);
        });
    }

    fn adjust_current_block(&mut self, deepen: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(session) = self.active_session.as_ref() else {
            return;
        };

        session.input.update(cx, |input, cx| {
            let text = input.text().to_string();
            if let Some(updated) = adjust_block_markup(&text, deepen) {
                input.set_value(updated, window, cx);
            }
        });
    }

    fn reload_conflict_from_disk(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(path) = self.document.path.clone() else {
            return;
        };

        let disk_text = match &self.document.conflict {
            ConflictState::Conflict { disk_text, .. } => disk_text.clone(),
            ConflictState::Clean => return,
        };
        let modified_at = std::fs::metadata(&path)
            .ok()
            .and_then(|meta| meta.modified().ok());
        self.clear_session();
        self.document
            .overwrite_from_disk_text(path, disk_text, modified_at);
        window.set_window_title(&self.window_title());
        self.set_status("Reloaded disk version");
        cx.notify();
    }

    fn keep_current_conflicted_version(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.document.keep_current_version();
        window.set_window_title(&self.window_title());
        self.set_status("Keeping current changes");
        cx.notify();
    }

    fn poll_workspace(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        for event in self.workspace.poll_events() {
            match event {
                WorkspaceEvent::Changed(path) => {
                    let Some(doc_path) = self.document.path.clone() else {
                        continue;
                    };
                    if path != doc_path || self.document.has_same_disk_timestamp(&path) {
                        continue;
                    }
                    let Ok(disk_text) = std::fs::read_to_string(&path) else {
                        continue;
                    };
                    let modified_at = std::fs::metadata(&path)
                        .ok()
                        .and_then(|meta| meta.modified().ok());
                    if self.document.dirty {
                        if self.document.text() != disk_text {
                            self.document.mark_conflict(disk_text, modified_at);
                            self.set_status("External changes detected");
                        }
                    } else {
                        self.clear_session();
                        self.document.overwrite_from_disk_text(
                            path.clone(),
                            disk_text,
                            modified_at,
                        );
                        window.set_window_title(&self.window_title());
                        self.set_status(format!("Reloaded {}", path.display()));
                    }
                }
                WorkspaceEvent::Removed(path) => {
                    if self.document.path.as_ref() == Some(&path) {
                        self.set_status(format!("File removed: {}", path.display()));
                    }
                }
                WorkspaceEvent::Unknown => {}
            }
        }
        cx.notify();
    }

    fn render_toolbar_button(
        &self,
        id: &'static str,
        label: &'static str,
        on_click: impl Fn(&Entity<Self>, &mut Window, &mut App) + 'static,
        cx: &Context<Self>,
    ) -> Button {
        let view = cx.entity();
        Button::new(id)
            .label(label)
            .ghost()
            .compact()
            .on_click(move |_, window, cx| on_click(&view, window, cx))
    }

    fn render_sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let selected_path = self.workspace.selected_file.clone();

        div()
            .size_full()
            .bg(cx.theme().secondary.opacity(0.45))
            .border_r_1()
            .border_color(cx.theme().sidebar_border)
            .p_2()
            .child(
                tree(&self.tree_state, move |ix, entry, selected, _, _| {
                    let path = PathBuf::from(entry.item().id.as_ref());
                    let label = if entry.is_folder() {
                        if entry.is_expanded() {
                            format!("▾ {}", entry.item().label)
                        } else {
                            format!("▸ {}", entry.item().label)
                        }
                    } else {
                        entry.item().label.to_string()
                    };

                    let is_selected_file = selected_path.as_ref() == Some(&path);
                    ListItem::new(ix)
                        .selected(selected || is_selected_file)
                        .pl(px(8. + entry.depth() as f32 * 14.))
                        .child(label)
                        .on_click({
                            let view = view.clone();
                            move |_, window, cx| {
                                if path.is_file() {
                                    let _ = view.update(cx, |this, cx| {
                                        this.open_file(path.clone(), window, cx);
                                    });
                                }
                            }
                        })
                })
                .size_full(),
            )
    }

    fn render_conflict_banner(&self, cx: &Context<Self>) -> Option<impl IntoElement> {
        if !matches!(self.document.conflict, ConflictState::Conflict { .. }) {
            return None;
        }

        let view = cx.entity();
        Some(
            div()
                .flex()
                .justify_between()
                .items_center()
                .gap_3()
                .px_4()
                .py_3()
                .mb_3()
                .rounded(px(10.))
                .bg(cx.theme().warning.opacity(0.14))
                .border_1()
                .border_color(cx.theme().warning.opacity(0.4))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child("External file changes detected")
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child("Reload the disk version or keep your current in-memory changes."),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            Button::new("reload-disk")
                                .label("Reload Disk Version")
                                .warning()
                                .compact()
                                .on_click({
                                    let view = view.clone();
                                    move |_, window, cx| {
                                        let _ = view.update(cx, |this, cx| {
                                            this.reload_conflict_from_disk(window, cx);
                                        });
                                    }
                                }),
                        )
                        .child(
                            Button::new("keep-current")
                                .label("Keep Current Changes")
                                .ghost()
                                .compact()
                                .on_click(move |_, window, cx| {
                                    let _ = view.update(cx, |this, cx| {
                                        this.keep_current_conflicted_version(window, cx);
                                    });
                                }),
                        ),
                ),
        )
    }

    fn render_empty_state(&self, cx: &Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .items_center()
            .justify_center()
            .size_full()
            .text_color(cx.theme().muted_foreground)
            .child("Open a Markdown file or folder to start writing.")
            .child(
                div()
                    .text_sm()
                    .child("Vellum v1 uses block-level hybrid editing."),
            )
    }

    fn render_block_row(
        &mut self,
        block_ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let block = self.document.blocks[block_ix].clone();
        let block_text = self.document.block_text(&block);
        let is_active = self
            .active_session
            .as_ref()
            .map(|session| session.block_id == block.id)
            .unwrap_or(false);
        let view = cx.entity();

        let content = if is_active {
            let session = self.active_session.as_ref().expect("active session");
            let input = style_active_input_for_block(
                Input::new(&session.input)
                    .appearance(false)
                    .bordered(false)
                    .focus_bordered(false),
                &block.kind,
            );
            div()
                .rounded(px(10.))
                .bg(cx.theme().secondary.opacity(0.22))
                .border_1()
                .border_color(cx.theme().accent.opacity(0.35))
                .px_3()
                .py_2()
                .child(input)
                .into_any_element()
        } else if self.document.is_empty() && block_text.is_empty() {
            div()
                .rounded(px(10.))
                .px_4()
                .py_4()
                .text_color(cx.theme().muted_foreground)
                .child("Start writing...")
                .into_any_element()
        } else {
            div()
                .rounded(px(10.))
                .px_4()
                .py_3()
                .hover(|style| style.bg(cx.theme().secondary.opacity(0.16)))
                .child(TextView::markdown(
                    ("preview", block.id),
                    block_text,
                    window,
                    cx,
                ))
                .into_any_element()
        };

        div()
            .id(("block-row", block.id))
            .w_full()
            .py_1()
            .child(
                div()
                    .id(("activate-block", block.id))
                    .w_full()
                    .rounded(px(10.))
                    .on_click(move |_, window, cx| {
                        let _ = view.update(cx, |this, cx| {
                            this.activate_block(block_ix, window, cx);
                        });
                    })
                    .child(content),
            )
            .into_any_element()
    }

    fn block_item_sizes(&self) -> Rc<Vec<gpui::Size<gpui::Pixels>>> {
        Rc::new(
            self.document
                .blocks
                .iter()
                .map(|block| {
                    let text = self.document.block_text(block);
                    let line_count = cmp::max(text.lines().count(), 1);
                    let base = match block.kind {
                        BlockKind::Heading { depth: 1 } => 52.,
                        BlockKind::Heading { depth: 2 } => 46.,
                        BlockKind::Heading { depth: 3 } => 42.,
                        BlockKind::Heading { depth: 4 } => 38.,
                        BlockKind::CodeFence { .. } => 54.,
                        BlockKind::Table => 58.,
                        _ => 34.,
                    };
                    let per_line = match block.kind {
                        BlockKind::Heading { depth: 1 } => 32.,
                        BlockKind::Heading { depth: 2 } => 28.,
                        BlockKind::Heading { depth: 3 } => 26.,
                        BlockKind::Heading { depth: 4 } => 24.,
                        BlockKind::CodeFence { .. } => 20.,
                        _ => 24.,
                    };
                    size(px(1.), px(base + line_count as f32 * per_line))
                })
                .collect(),
        )
    }

    fn render_editor(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        if self.document.blocks.is_empty() {
            return self.render_empty_state(cx).into_any_element();
        }

        let view = cx.entity();
        let sizes = self.block_item_sizes();
        let conflict_banner = self
            .render_conflict_banner(cx)
            .map(|banner| banner.into_any_element());

        div()
            .size_full()
            .overflow_hidden()
            .child(
                div()
                    .size_full()
                    .px_6()
                    .py_5()
                    .when_some(conflict_banner, |this, banner| this.child(banner))
                    .child(
                        div()
                            .mx_auto()
                            .max_w(px(MAX_EDITOR_WIDTH))
                            .w_full()
                            .h_full()
                            .child(
                                v_virtual_list(
                                    view,
                                    "document-blocks",
                                    sizes,
                                    |this, range: Range<usize>, window, cx| {
                                        range
                                            .map(|ix| this.render_block_row(ix, window, cx))
                                            .collect::<Vec<_>>()
                                    },
                                )
                                .size_full(),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn render_status_bar(&self, cx: &Context<Self>) -> impl IntoElement {
        let workspace = self
            .app_state
            .workspace_root
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "No workspace".to_string());
        let doc_status = if matches!(self.document.conflict, ConflictState::Conflict { .. }) {
            "Conflict"
        } else if self.document.saving {
            "Saving"
        } else if self.document.dirty {
            "Dirty"
        } else {
            "Saved"
        };

        div()
            .flex()
            .justify_between()
            .items_center()
            .gap_4()
            .px_4()
            .py_2()
            .border_t_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .text_sm()
            .child(
                div()
                    .flex()
                    .gap_3()
                    .child(workspace)
                    .child(format!("Doc {}", self.document.display_name()))
                    .child(format!("State {doc_status}"))
                    .child(format!("Blocks {}", self.document.blocks.len()))
                    .child(format!(
                        "Active {}",
                        self.app_state.active_document_id.unwrap_or_default()
                    )),
            )
            .child(
                div()
                    .text_color(cx.theme().muted_foreground)
                    .child(self.status_message.clone()),
            )
    }

    fn on_open_file(&mut self, _: &OpenFile, window: &mut Window, cx: &mut Context<Self>) {
        self.open_file_dialog(window, cx);
    }

    fn on_open_folder(&mut self, _: &OpenFolder, window: &mut Window, cx: &mut Context<Self>) {
        self.open_folder_dialog(window, cx);
    }

    fn on_new_file(&mut self, _: &NewFile, window: &mut Window, cx: &mut Context<Self>) {
        self.create_new_file(window, cx);
    }

    fn on_save_now(&mut self, _: &SaveNow, window: &mut Window, cx: &mut Context<Self>) {
        if let Err(err) = self.save_document(window, cx) {
            self.set_status(format!("Save failed: {err}"));
        }
    }

    fn on_save_as(&mut self, _: &SaveAs, window: &mut Window, cx: &mut Context<Self>) {
        if let Err(err) = self.save_document_as(window, cx) {
            self.set_status(format!("Save As failed: {err}"));
        }
    }

    fn on_toggle_sidebar(&mut self, _: &ToggleSidebar, _: &mut Window, cx: &mut Context<Self>) {
        self.sidebar_visible = !self.sidebar_visible;
        cx.notify();
    }

    fn on_bold_selection(
        &mut self,
        _: &BoldSelection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.apply_markup("**", "**", "bold text", window, cx);
    }

    fn on_italic_selection(
        &mut self,
        _: &ItalicSelection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.apply_markup("*", "*", "italic text", window, cx);
    }

    fn on_link_selection(
        &mut self,
        _: &LinkSelection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.apply_markup("[", "](https://)", "link text", window, cx);
    }

    fn on_promote_block(&mut self, _: &PromoteBlock, window: &mut Window, cx: &mut Context<Self>) {
        self.adjust_current_block(false, window, cx);
    }

    fn on_demote_block(&mut self, _: &DemoteBlock, window: &mut Window, cx: &mut Context<Self>) {
        self.adjust_current_block(true, window, cx);
    }

    fn on_exit_block_edit(
        &mut self,
        _: &ExitBlockEdit,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.exit_edit_mode(window, cx);
    }

    fn on_focus_prev_block(
        &mut self,
        _: &FocusPrevBlock,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.focus_adjacent_block(-1, window, cx);
    }

    fn on_focus_next_block(
        &mut self,
        _: &FocusNextBlock,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.focus_adjacent_block(1, window, cx);
    }
}

fn next_untitled_path(root: &Path) -> PathBuf {
    let mut index = 1usize;
    loop {
        let candidate = if index == 1 {
            root.join("untitled.md")
        } else {
            root.join(format!("untitled-{index}.md"))
        };
        if !candidate.exists() {
            return candidate;
        }
        index += 1;
    }
}

fn position_for_byte_offset(text: &str, byte_offset: usize) -> (usize, usize) {
    let clamped = cmp::min(byte_offset, text.len());
    let prefix = &text[..clamped];
    let row = prefix.bytes().filter(|byte| *byte == b'\n').count();
    let col = prefix
        .rsplit_once('\n')
        .map(|(_, tail)| tail.chars().count())
        .unwrap_or_else(|| prefix.chars().count());
    (row, col)
}

fn activation_cursor_offset(text: &str) -> usize {
    text.trim_end_matches(['\r', '\n']).len()
}

fn style_active_input_for_block(input: Input, kind: &BlockKind) -> Input {
    match kind {
        BlockKind::Heading { depth: 1 } => input.text_size(px(34.)).line_height(px(40.)),
        BlockKind::Heading { depth: 2 } => input.text_size(px(28.)).line_height(px(34.)),
        BlockKind::Heading { depth: 3 } => input.text_size(px(24.)).line_height(px(30.)),
        BlockKind::Heading { depth: 4 } => input.text_size(px(20.)).line_height(px(26.)),
        BlockKind::Heading { .. } => input.text_base(),
        _ => input,
    }
}

fn adjust_block_markup(text: &str, deepen: bool) -> Option<String> {
    let mut lines = text.lines();
    let first = lines.next()?;
    let rest = if text.contains('\n') {
        text[first.len()..].to_string()
    } else {
        String::new()
    };

    let trimmed = first.trim_start();
    let indent = &first[..first.len().saturating_sub(trimmed.len())];

    if let Some(space_ix) = trimmed.find(' ') {
        let marker = &trimmed[..space_ix];
        if marker.chars().all(|ch| ch == '#') && !marker.is_empty() {
            let current = marker.len();
            let updated = if deepen {
                cmp::min(current + 1, 6)
            } else {
                current.saturating_sub(1)
            };
            let head = if updated == 0 {
                format!("{indent}{}", &trimmed[space_ix + 1..])
            } else {
                format!(
                    "{indent}{} {}",
                    "#".repeat(updated),
                    &trimmed[space_ix + 1..]
                )
            };
            return Some(format!("{head}{rest}"));
        }
    }

    let list_markers = ["- ", "* ", "+ ", "- [ ] ", "- [x] ", "* [ ] ", "* [x] "];
    if list_markers
        .iter()
        .any(|marker| trimmed.starts_with(marker))
        || trimmed
            .split_once(". ")
            .map(|(n, _)| n.chars().all(|ch| ch.is_ascii_digit()))
            .unwrap_or(false)
    {
        let updated_indent = if deepen {
            format!("{indent}  ")
        } else if indent.len() >= 2 {
            indent[..indent.len() - 2].to_string()
        } else {
            String::new()
        };

        let updated = text
            .lines()
            .map(|line| format!("{updated_indent}{}", line.trim_start()))
            .collect::<Vec<_>>()
            .join("\n");
        return Some(updated);
    }

    if deepen {
        Some(format!("# {text}"))
    } else {
        None
    }
}

impl Render for VellumApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.set_window_title(&self.window_title());

        let body = h_resizable("vellum-layout")
            .child(
                resizable_panel()
                    .size(px(260.))
                    .size_range(px(180.)..px(420.))
                    .visible(self.sidebar_visible)
                    .child(self.render_sidebar(cx)),
            )
            .child(resizable_panel().child(self.render_editor(window, cx)));

        div()
            .id("vellum-app")
            .key_context(APP_CONTEXT)
            .size_full()
            .flex()
            .flex_col()
            .on_action(cx.listener(Self::on_open_file))
            .on_action(cx.listener(Self::on_open_folder))
            .on_action(cx.listener(Self::on_new_file))
            .on_action(cx.listener(Self::on_save_now))
            .on_action(cx.listener(Self::on_save_as))
            .on_action(cx.listener(Self::on_toggle_sidebar))
            .on_action(cx.listener(Self::on_bold_selection))
            .on_action(cx.listener(Self::on_italic_selection))
            .on_action(cx.listener(Self::on_link_selection))
            .on_action(cx.listener(Self::on_promote_block))
            .on_action(cx.listener(Self::on_demote_block))
            .on_action(cx.listener(Self::on_exit_block_edit))
            .on_action(cx.listener(Self::on_focus_prev_block))
            .on_action(cx.listener(Self::on_focus_next_block))
            .child(
                TitleBar::new().child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_4()
                        .w_full()
                        .pr_3()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(self.render_toolbar_button(
                                    "open-folder",
                                    "Open Folder",
                                    |view, window, cx| {
                                        let _ = view.update(cx, |this, cx| {
                                            this.open_folder_dialog(window, cx)
                                        });
                                    },
                                    cx,
                                ))
                                .child(self.render_toolbar_button(
                                    "open-file",
                                    "Open File",
                                    |view, window, cx| {
                                        let _ = view.update(cx, |this, cx| {
                                            this.open_file_dialog(window, cx)
                                        });
                                    },
                                    cx,
                                ))
                                .child(self.render_toolbar_button(
                                    "new-file",
                                    "New File",
                                    |view, window, cx| {
                                        let _ = view.update(cx, |this, cx| {
                                            this.create_new_file(window, cx)
                                        });
                                    },
                                    cx,
                                ))
                                .child(self.render_toolbar_button(
                                    "save-now",
                                    "Save",
                                    |view, window, cx| {
                                        let _ = view.update(cx, |this, cx| {
                                            let _ = this.save_document(window, cx);
                                        });
                                    },
                                    cx,
                                ))
                                .child(self.render_toolbar_button(
                                    "save-as",
                                    "Save As",
                                    |view, window, cx| {
                                        let _ = view.update(cx, |this, cx| {
                                            let _ = this.save_document_as(window, cx);
                                        });
                                    },
                                    cx,
                                ))
                                .child(self.render_toolbar_button(
                                    "toggle-sidebar",
                                    "Toggle Sidebar",
                                    |view, _, cx| {
                                        let _ = view.update(cx, |this, cx| {
                                            this.sidebar_visible = !this.sidebar_visible;
                                            cx.notify();
                                        });
                                    },
                                    cx,
                                )),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_3()
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(self.document.display_name()),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(if self.document.dirty {
                                            "Unsaved"
                                        } else {
                                            "Synced"
                                        }),
                                ),
                        ),
                ),
            )
            .child(div().flex_1().min_h(px(0.)).child(body))
            .child(self.render_status_bar(cx))
    }
}
