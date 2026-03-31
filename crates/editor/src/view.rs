use crate::layout::{
    activation_cursor_offset, adjust_block_markup, block_layout_metrics, count_document_words,
    markdown_preview_style, position_for_byte_offset, style_active_input_for_block,
};
use crate::*;

#[derive(Debug, Clone, Default)]
pub struct EditorSnapshot {
    pub path: Option<PathBuf>,
    pub display_name: String,
    pub dirty: bool,
    pub saving: bool,
    pub has_conflict: bool,
    pub word_count: usize,
    pub status_message: SharedString,
}

#[derive(Debug, Clone)]
pub enum EditorEvent {
    Changed(EditorSnapshot),
}

pub struct MarkdownEditor {
    document: DocumentState,
    active_session: Option<ActiveBlockSession>,
    input_subscription: Option<Subscription>,
    flush_generation: u64,
    autosave_generation: u64,
    status_message: SharedString,
}

impl EventEmitter<EditorEvent> for MarkdownEditor {}

impl MarkdownEditor {
    pub fn new(_: &mut Window, _: &mut Context<Self>) -> Self {
        Self {
            document: DocumentState::new_empty(None, None),
            active_session: None,
            input_subscription: None,
            flush_generation: 0,
            autosave_generation: 0,
            status_message: SharedString::from(""),
        }
    }

    #[cfg(test)]
    fn new_for_test(document: DocumentState) -> Self {
        Self {
            document,
            active_session: None,
            input_subscription: None,
            flush_generation: 0,
            autosave_generation: 0,
            status_message: SharedString::from(""),
        }
    }

    pub fn snapshot(&self) -> EditorSnapshot {
        EditorSnapshot {
            path: self.document.path.clone(),
            display_name: self.document.display_name(),
            dirty: self.document.dirty,
            saving: self.document.saving,
            has_conflict: matches!(self.document.conflict, ConflictState::Conflict { .. }),
            word_count: count_document_words(&self.document.text()),
            status_message: self.status_message.clone(),
        }
    }

    pub fn current_document_dir(&self) -> Option<PathBuf> {
        self.document
            .suggested_path()
            .and_then(|path| path.parent().map(Path::to_path_buf))
    }

    pub fn document_path(&self) -> Option<&PathBuf> {
        self.document.path.as_ref()
    }

    pub fn open_path(
        &mut self,
        path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        self.flush_active_session(true, window, cx)?;
        self.clear_session();
        self.document = DocumentState::from_disk(path.clone())?;
        self.set_status(format!("Opened {}", path.display()));
        self.emit_changed(cx);
        Ok(())
    }

    pub fn new_untitled(
        &mut self,
        suggested_path: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Err(err) = self.flush_active_session(true, window, cx) {
            self.set_status(format!("Failed to flush before new file: {err}"));
        }

        self.clear_session();
        self.document = DocumentState::new_empty(None, suggested_path);
        self.set_status("New file");
        self.emit_changed(cx);
    }

    pub fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Result<()> {
        self.flush_active_session(false, window, cx)?;
        self.document.save_now()?;
        self.set_status(format!("Saved {}", self.document.display_name()));
        self.emit_changed(cx);
        Ok(())
    }

    pub fn save_as(
        &mut self,
        path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        self.flush_active_session(false, window, cx)?;
        self.document.set_path(path.clone());
        self.document.save_now()?;
        self.set_status(format!("Saved {}", path.display()));
        self.emit_changed(cx);
        Ok(())
    }

    pub fn handle_disk_change(
        &mut self,
        path: PathBuf,
        disk_text: String,
        modified_at: Option<SystemTime>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.document.path.as_ref() != Some(&path)
            || self.document.has_same_disk_timestamp(&path)
        {
            return;
        }

        if self.document.dirty {
            if self.document.text() != disk_text {
                self.document.mark_conflict(disk_text, modified_at);
                self.set_status("External changes detected");
                self.emit_changed(cx);
            }
            return;
        }

        self.clear_session();
        self.document
            .overwrite_from_disk_text(path.clone(), disk_text, modified_at);
        self.set_status(format!("Reloaded {}", path.display()));
        self.emit_changed(cx);
    }

    pub fn handle_disk_removed(&mut self, path: PathBuf, _: &mut Window, cx: &mut Context<Self>) {
        if self.document.path.as_ref() == Some(&path) {
            self.set_status(format!("File removed: {}", path.display()));
            self.emit_changed(cx);
        }
    }

    fn set_status(&mut self, status: impl Into<SharedString>) {
        self.status_message = status.into();
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
                    self.emit_changed(cx);
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
                        let _ = this.save(window, cx);
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
        self.emit_changed(cx);
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

        let new_block_ix = self.document.block_index_at_offset(new_anchor);
        let new_block = self.document.blocks[new_block_ix].clone();
        let new_text = self.document.block_text(&new_block);
        let new_cursor_offset = new_anchor.saturating_sub(new_block.byte_range.start);

        self.input_subscription = None;
        if exit_after_flush {
            self.active_session = None;
            self.set_status("Block synced");
            self.emit_changed(cx);
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
        self.emit_changed(cx);
        Ok(())
    }

    fn current_block_index(&self) -> Option<usize> {
        self.active_session
            .as_ref()
            .and_then(|session| self.document.block_index_by_id(session.block_id))
    }

    pub(crate) fn focus_adjacent_block(
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

    pub(crate) fn exit_edit_mode(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Err(err) = self.flush_active_session(true, window, cx) {
            self.set_status(format!("Failed to exit edit mode: {err}"));
            self.emit_changed(cx);
        }
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

    pub(crate) fn adjust_current_block(
        &mut self,
        deepen: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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

    fn reload_conflict_from_disk(&mut self, _: &mut Window, cx: &mut Context<Self>) {
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
        self.set_status("Reloaded disk version");
        self.emit_changed(cx);
    }

    fn keep_current_conflicted_version(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.document.keep_current_version();
        self.set_status("Keeping current changes");
        self.emit_changed(cx);
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
        let block = self.document.blocks[block_ix].clone();
        let block_text = self.document.block_text(&block);
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
        } else if self.document.is_empty() && block_text.is_empty() {
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
                    TextView::markdown(("preview", block.id), block_text, window, cx)
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
            self.document
                .blocks
                .iter()
                .map(|block| {
                    let text = self.document.block_text(block);
                    let line_count = cmp::max(text.lines().count(), 1);
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
        let content = if self.document.blocks.is_empty() {
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
            .child(self.render_editor(window, cx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_reflects_document_state() {
        let mut editor = MarkdownEditor::new_for_test(DocumentState::from_text(
            Some(PathBuf::from("note.md")),
            Some(PathBuf::from("note.md")),
            "hello world",
        ));
        editor.document.dirty = true;
        editor.document.saving = true;
        editor.document.mark_conflict("disk".to_string(), None);
        editor.set_status("Testing");

        let snapshot = editor.snapshot();
        assert_eq!(snapshot.path, Some(PathBuf::from("note.md")));
        assert_eq!(snapshot.display_name, "note.md");
        assert!(snapshot.dirty);
        assert!(snapshot.saving);
        assert!(snapshot.has_conflict);
        assert_eq!(snapshot.word_count, 2);
        assert_eq!(snapshot.status_message, SharedString::from("Testing"));
    }
}
