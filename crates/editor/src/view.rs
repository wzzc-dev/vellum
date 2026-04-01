use std::time::SystemTime;

use crate::layout::{
    block_layout_metrics, markdown_preview_style, position_for_byte_offset,
    style_active_input_for_block,
};
use crate::*;
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
            let input = cx.new(|cx| {
                let mut state = match &block.kind {
                    BlockKind::CodeFence { language } => InputState::new(window, cx)
                        .code_editor(language.clone().unwrap_or_else(|| "text".to_string()))
                        .line_number(false),
                    _ => InputState::new(window, cx)
                        .multi_line(true)
                        .auto_grow(1, 24),
                };
                state.set_value(block.text.clone(), window, cx);
                state
            });
            let view = cx.entity();
            let subscription =
                window.subscribe(&input, cx, move |_, event: &InputEvent, window, cx| {
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
            session.input.update(cx, |input, cx| {
                let current_text = input.text().to_string();
                let current_cursor = input.cursor();
                if current_text != block.text {
                    input.set_value(block.text.clone(), window, cx);
                }
                if current_cursor != desired_cursor {
                    let (row, col) = position_for_byte_offset(&block.text, desired_cursor);
                    input.set_cursor_position(
                        gpui_component::input::Position {
                            line: row as u32,
                            character: col as u32,
                        },
                        window,
                        cx,
                    );
                }
                input.focus(window, cx);
            });
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
                let (text, cursor_offset) = session
                    .input
                    .update(cx, |input, _| (input.text().to_string(), input.cursor()));
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

    fn activate_block(&mut self, block_ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        let effects = self
            .controller
            .dispatch(EditCommand::ActivateBlock(block_ix));
        self.apply_effects(window, cx, effects);
    }

    pub(crate) fn focus_adjacent_block(
        &mut self,
        direction: isize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let effects = self
            .controller
            .dispatch(EditCommand::FocusAdjacentBlock { direction });
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

        let (selection, cursor_offset) = session.input.update(cx, |input, cx| {
            let selection = input
                .selected_text_range(true, window, cx)
                .and_then(|selection| {
                    if selection.range.is_empty() {
                        None
                    } else {
                        Some(selection.range)
                    }
                });
            (selection, input.cursor())
        });

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
        let is_active = self
            .active_session
            .as_ref()
            .map(|session| session.block_id == block.id)
            .unwrap_or(false);
        let view = cx.entity();
        let metrics = block_layout_metrics(&block.kind);

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
                .px_1()
                .py(px(metrics.block_padding_y))
                .child(input)
                .into_any_element()
        } else if self.snapshot.blocks.len() == 1 && block.text.is_empty() {
            div()
                .px_1()
                .py(px(metrics.block_padding_y + 6.))
                .text_size(px(BODY_FONT_SIZE))
                .line_height(px(BODY_LINE_HEIGHT))
                .text_color(cx.theme().muted_foreground)
                .child("Start writing...")
                .into_any_element()
        } else {
            div()
                .px_1()
                .py(px(metrics.block_padding_y))
                .text_size(px(BODY_FONT_SIZE))
                .line_height(px(BODY_LINE_HEIGHT))
                .child(
                    TextView::markdown(("preview", block.id), block.text, window, cx)
                        .style(markdown_preview_style()),
                )
                .into_any_element()
        };

        div()
            .id(("block-row", block.id))
            .w_full()
            .py(px(metrics.row_spacing_y))
            .child(
                div()
                    .id(("activate-block", block.id))
                    .w_full()
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
            self.snapshot
                .blocks
                .iter()
                .map(|block| {
                    let line_count = cmp::max(block.text.lines().count(), 1);
                    let metrics = block_layout_metrics(&block.kind);
                    size(
                        px(1.),
                        px(metrics.block_padding_y * 2.
                            + metrics.row_spacing_y * 2.
                            + metrics.line_height * line_count as f32
                            + metrics.extra_height),
                    )
                })
                .collect(),
        )
    }

    fn render_editor(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let view = cx.entity();
        let sizes = self.block_item_sizes();
        let conflict_banner = self
            .render_conflict_banner(cx)
            .map(|banner| banner.into_any_element());
        let content = if self.snapshot.blocks.is_empty() {
            self.render_empty_state(cx).into_any_element()
        } else {
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
            .size_full()
            .into_any_element()
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
