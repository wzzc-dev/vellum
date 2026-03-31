use super::layout::{activation_cursor_offset, adjust_block_markup, position_for_byte_offset};
use super::*;

impl VellumApp {
    pub(super) fn handle_input_event(
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

    pub(super) fn schedule_flush(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

    pub(super) fn schedule_autosave(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

    pub(super) fn activate_block(
        &mut self,
        block_ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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

    pub(super) fn flush_active_session(
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

    pub(super) fn current_block_index(&self) -> Option<usize> {
        self.active_session
            .as_ref()
            .and_then(|session| self.document.block_index_by_id(session.block_id))
    }

    pub(super) fn focus_adjacent_block(
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

    pub(super) fn exit_edit_mode(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Err(err) = self.flush_active_session(true, window, cx) {
            self.set_status(format!("Failed to exit edit mode: {err}"));
        }
    }

    pub(super) fn apply_markup(
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

    pub(super) fn adjust_current_block(
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

    pub(super) fn reload_conflict_from_disk(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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

    pub(super) fn keep_current_conflicted_version(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.document.keep_current_version();
        window.set_window_title(&self.window_title());
        self.set_status("Keeping current changes");
        cx.notify();
    }

    pub(super) fn poll_workspace(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
}
