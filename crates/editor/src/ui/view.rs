use std::{
    cell::RefCell,
    collections::HashMap,
    path::PathBuf,
    rc::Rc,
    time::SystemTime,
};

use anyhow::Result;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, Bounds, ClickEvent, Context, EventEmitter, InteractiveElement, IntoElement,
    KeyDownEvent, ParentElement, Pixels, Render, StatefulInteractiveElement, Styled,
    Subscription, Timer, VisualContext, Window, canvas, div, px,
};
use gpui_component::ActiveTheme;

use crate::core::controller::{
    DocumentSource, EditCommand, EditorController, EditorEffects, EditorSnapshot, FileSyncEvent,
    SyncPolicy,
};

use super::{
    BODY_FONT_SIZE, BODY_LINE_HEIGHT, EDITOR_CONTEXT, MAX_EDITOR_WIDTH,
    component_ui::{
        BlockInput, Button, ButtonVariants as _, InputEvent, render_block_list,
        render_markdown_preview,
    },
    layout::{block_layout_metrics, byte_offset_for_click_position},
    session::ActiveBlockSession,
};
#[derive(Debug, Clone)]
pub enum EditorEvent {
    Changed(EditorSnapshot),
}

pub struct MarkdownEditor {
    controller: EditorController,
    snapshot: EditorSnapshot,
    active_session: Option<ActiveBlockSession>,
    input_subscription: Option<Subscription>,
    autosave_generation: u64,
    block_bounds: Rc<RefCell<HashMap<u64, Bounds<Pixels>>>>,
}

impl EventEmitter<EditorEvent> for MarkdownEditor {}

impl MarkdownEditor {
    pub fn new(_: &mut Window, _: &mut Context<Self>) -> Self {
        let controller = EditorController::new(
            DocumentSource::Empty {
                suggested_path: None,
            },
            SyncPolicy::default(),
        );
        let snapshot = controller.snapshot();
        Self {
            controller,
            snapshot,
            active_session: None,
            input_subscription: None,
            autosave_generation: 0,
            block_bounds: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    pub fn snapshot(&self) -> EditorSnapshot {
        self.snapshot.clone()
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

    fn apply_effects(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        effects: EditorEffects,
    ) {
        if effects.active_block_changed {
            self.input_subscription = None;
            if self.snapshot.active_block_id != self.controller.snapshot().active_block_id {
                self.active_session = None;
            }
        }

        self.snapshot = self.controller.snapshot();
        self.sync_active_input(window, cx);
        if effects.changed || effects.active_block_changed {
            self.emit_changed(cx);
        }
    }

    fn emit_changed(&mut self, cx: &mut Context<Self>) {
        let snapshot = self.snapshot();
        cx.emit(EditorEvent::Changed(snapshot));
        cx.notify();
    }

    fn clear_session(&mut self) {
        self.input_subscription = None;
        self.active_session = None;
    }

    fn sync_active_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(block_id) = self.snapshot.active_block_id else {
            self.clear_session();
            return;
        };
        let Some(block) = self.snapshot.block_by_id(block_id).cloned() else {
            self.clear_session();
            return;
        };

        let needs_new_input = self
            .active_session
            .as_ref()
            .map(|session| session.block_id != block_id)
            .unwrap_or(true);

        if needs_new_input {
            self.input_subscription = None;
            let input = BlockInput::new(&block.kind, block.text.clone(), window, cx);
            let view = cx.entity();
            let subscription =
                window.subscribe(input.entity(), cx, move |_, event: &InputEvent, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.handle_input_event(event, window, cx);
                    });
                });
            self.active_session = Some(ActiveBlockSession::new(block_id, input));
            self.input_subscription = Some(subscription);
        }

        let desired_cursor = self
            .snapshot
            .active_cursor_offset
            .unwrap_or_else(|| block.text.len());
        if let Some(session) = self.active_session.as_ref() {
            session.input.sync(&block.text, desired_cursor, window, cx);
        }
    }

    fn handle_input_event(
        &mut self,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::Change => {
                let Some(session) = self.active_session.as_ref() else {
                    return;
                };
                let (text, cursor_offset) = session.input.text_and_cursor(cx);
                let effects = self.controller.dispatch(EditCommand::ReplaceActiveBlock {
                    text,
                    cursor_offset,
                });
                self.apply_effects(window, cx, effects);
                self.schedule_autosave(window, cx);
            }
            InputEvent::Blur | InputEvent::Focus | InputEvent::PressEnter { .. } => {}
        }
    }

    fn schedule_autosave(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.autosave_generation = self.autosave_generation.wrapping_add(1);
        let token = self.autosave_generation;
        let view = cx.entity();
        let autosave_delay = self.controller.autosave_delay();
        window
            .spawn(cx, async move |cx| {
                Timer::after(autosave_delay).await;
                let _ = cx.update_window_entity(&view, |this, window, cx| {
                    if this.autosave_generation == token && this.snapshot.dirty {
                        let _ = this.save(window, cx);
                    }
                });
            })
            .detach();
    }

    fn activate_block(
        &mut self,
        block_ix: usize,
        cursor_offset: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let effects = self
            .controller
            .dispatch(EditCommand::ActivateBlock {
                index: block_ix,
                cursor_offset,
            });
        self.apply_effects(window, cx, effects);
    }

    fn activate_block_from_click(
        &mut self,
        block_ix: usize,
        event: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let cursor_offset = self.snapshot.blocks.get(block_ix).cloned().and_then(|block| {
            self.block_bounds
                .borrow()
                .get(&block.id)
                .cloned()
                .map(|bounds| {
                    byte_offset_for_click_position(
                        &block.kind,
                        &block.text,
                        event.position(),
                        bounds,
                        window,
                    )
                })
        });

        self.activate_block(block_ix, cursor_offset, window, cx);
    }

    pub(crate) fn focus_adjacent_block(
        &mut self,
        direction: isize,
        preferred_column: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let effects = self
            .controller
            .dispatch(EditCommand::FocusAdjacentBlock {
                direction,
                preferred_column,
            });
        self.apply_effects(window, cx, effects);
    }

    pub(crate) fn exit_edit_mode(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let effects = self.controller.dispatch(EditCommand::ExitEditMode);
        self.apply_effects(window, cx, effects);
    }

    pub(crate) fn undo(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let effects = self.controller.dispatch(EditCommand::Undo);
        self.apply_effects(window, cx, effects);
    }

    pub(crate) fn redo(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let effects = self.controller.dispatch(EditCommand::Redo);
        self.apply_effects(window, cx, effects);
    }

    pub(crate) fn apply_markup(
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

        let (selection, cursor_offset) = session.input.selection_and_cursor(window, cx);

        let effects = self.controller.dispatch(EditCommand::WrapActiveSelection {
            selection,
            cursor_offset,
            before: before.to_string(),
            after: after.to_string(),
            placeholder: placeholder.to_string(),
        });
        self.apply_effects(window, cx, effects);
    }

    pub(crate) fn adjust_current_block(
        &mut self,
        deepen: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let effects = self
            .controller
            .dispatch(EditCommand::AdjustActiveBlock { deepen });
        self.apply_effects(window, cx, effects);
    }

    fn reload_conflict_from_disk(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let effects = self.controller.dispatch(EditCommand::ReloadConflict);
        self.apply_effects(window, cx, effects);
    }

    fn keep_current_conflicted_version(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let effects = self.controller.dispatch(EditCommand::KeepCurrentConflict);
        self.apply_effects(window, cx, effects);
    }

    fn handle_active_navigation_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let modifiers = event.keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.shift || modifiers.platform || modifiers.function {
            return false;
        }

        let direction = match event.keystroke.key.as_str() {
            "up" => -1,
            "down" => 1,
            _ => return false,
        };

        let Some(session) = self.active_session.as_ref() else {
            return false;
        };
        let state = session.input.navigation_state(window, cx);
        if state.has_selection {
            return false;
        }

        let last_line = state.text.lines().count().max(1).saturating_sub(1);
        let at_boundary = if direction < 0 {
            state.line == 0
        } else {
            state.line >= last_line
        };
        if !at_boundary {
            return false;
        }

        self.focus_adjacent_block(direction, Some(state.column), window, cx);
        true
    }

    fn capture_block_bounds(&self, block_id: u64) -> AnyElement {
        let block_bounds = self.block_bounds.clone();
        canvas(
            move |bounds, _, _| bounds,
            move |_, bounds, _, _| {
                block_bounds.borrow_mut().insert(block_id, bounds);
            },
        )
        .absolute()
        .size_full()
        .into_any_element()
    }

    fn render_conflict_banner(&self, cx: &Context<Self>) -> Option<impl IntoElement> {
        if !self.snapshot.has_conflict {
            return None;
        }

        let view = cx.entity();
        Some(
            div()
                .flex()
                .justify_between()
                .items_center()
                .gap_3()
                .px_3()
                .py_2()
                .mb_4()
                .rounded(px(8.))
                .bg(cx.theme().warning.opacity(0.08))
                .border_1()
                .border_color(cx.theme().warning.opacity(0.22))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_0p5()
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
            .pt(px(56.))
            .text_size(px(BODY_FONT_SIZE))
            .line_height(px(BODY_LINE_HEIGHT))
            .text_color(cx.theme().muted_foreground)
            .child("Open a Markdown file or press Ctrl+N to start writing.")
            .child(
                div()
                    .text_sm()
                    .child("Vellum keeps editing in a single quiet writing column."),
            )
    }

    fn render_block_row(
        &mut self,
        block_ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let block = self.snapshot.blocks[block_ix].clone();
        let block_is_empty = block.text.is_empty();
        let is_active = self
            .active_session
            .as_ref()
            .map(|session| session.block_id == block.id)
            .unwrap_or(false);
        let view = cx.entity();
        let metrics = block_layout_metrics(&block.kind);

        let content_body = if is_active {
            let session = self.active_session.as_ref().expect("active session");
            div()
                .w_full()
                .capture_key_down({
                    let view = view.clone();
                    move |event, window, cx| {
                        let handled =
                            view.update(cx, |this, cx| this.handle_active_navigation_key(event, window, cx));
                        if handled {
                            cx.stop_propagation();
                            window.prevent_default();
                        }
                    }
                })
                .child(session.input.render(&block.kind))
                .into_any_element()
        } else if self.snapshot.blocks.len() == 1 && block_is_empty {
            div()
                .text_size(px(BODY_FONT_SIZE))
                .line_height(px(BODY_LINE_HEIGHT))
                .text_color(cx.theme().muted_foreground)
                .child("Start writing...")
                .into_any_element()
        } else {
            div()
                .text_size(px(BODY_FONT_SIZE))
                .line_height(px(BODY_LINE_HEIGHT))
                .child(render_markdown_preview(block.id, block.text, window, cx))
                .into_any_element()
        };

        let placeholder_extra_padding = if !is_active && self.snapshot.blocks.len() == 1 && block_is_empty {
            6.
        } else {
            0.
        };
        let content = div()
            .px_1()
            .py(px(metrics.block_padding_y + placeholder_extra_padding))
            .child(
                div()
                    .relative()
                    .w_full()
                    .child(self.capture_block_bounds(block.id))
                    .child(content_body),
            )
            .into_any_element();

        div()
            .id(("block-row", block.id))
            .w_full()
            .py(px(metrics.row_spacing_y))
            .child(
                div()
                    .id(("activate-block", block.id))
                    .w_full()
                    .on_click(move |event, window, cx| {
                        let _ = view.update(cx, |this, cx| {
                            this.activate_block_from_click(block_ix, event, window, cx);
                        });
                    })
                    .child(content),
            )
            .into_any_element()
    }

    fn render_editor(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        self.block_bounds.borrow_mut().clear();
        let conflict_banner = self
            .render_conflict_banner(cx)
            .map(|banner| banner.into_any_element());
        let content = if self.snapshot.blocks.is_empty() {
            self.render_empty_state(cx).into_any_element()
        } else {
            render_block_list(
                (0..self.snapshot.blocks.len())
                    .map(|ix| self.render_block_row(ix, _window, cx))
                    .collect::<Vec<_>>(),
            )
        };

        div()
            .size_full()
            .min_w(px(0.))
            .min_h(px(0.))
            .bg(cx.theme().background)
            .overflow_hidden()
            .child(
                div()
                    .size_full()
                    .min_w(px(0.))
                    .min_h(px(0.))
                    .flex()
                    .flex_col()
                    .px_8()
                    .pt(px(28.))
                    .pb(px(44.))
                    .when_some(conflict_banner, |this, banner| this.child(banner))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .min_h(px(0.))
                            .mx_auto()
                            .max_w(px(MAX_EDITOR_WIDTH))
                            .w_full()
                            .child(content),
                    ),
            )
            .into_any_element()
    }
}

impl Render for MarkdownEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("markdown-editor")
            .key_context(EDITOR_CONTEXT)
            .size_full()
            .min_w(px(0.))
            .min_h(px(0.))
            .bg(cx.theme().background)
            .on_action(cx.listener(Self::on_bold_selection))
            .on_action(cx.listener(Self::on_italic_selection))
            .on_action(cx.listener(Self::on_link_selection))
            .on_action(cx.listener(Self::on_promote_block))
            .on_action(cx.listener(Self::on_demote_block))
            .on_action(cx.listener(Self::on_exit_block_edit))
            .on_action(cx.listener(Self::on_focus_prev_block))
            .on_action(cx.listener(Self::on_focus_next_block))
            .on_action(cx.listener(Self::on_undo_edit))
            .on_action(cx.listener(Self::on_redo_edit))
            .child(self.render_editor(window, cx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_is_forwarded_from_controller() {
        let controller = EditorController::new(
            DocumentSource::Text {
                path: Some(PathBuf::from("note.md")),
                suggested_path: Some(PathBuf::from("note.md")),
                text: "hello world".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        let snapshot = controller.snapshot();
        assert_eq!(snapshot.display_name, "note.md");
        assert_eq!(snapshot.word_count, 2);
    }
}
