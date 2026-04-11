use gpui::{Context, EntityInputHandler as _, Window};
use gpui_component::input::{InputEvent, InputState, Position};

use crate::{
    BlockKind, EditCommand, RenderSpanKind, SelectionAffinity, SelectionState,
    core::{
        controller::EditorSnapshot,
        text_ops::{clamp_to_char_boundary, compute_document_diff, line_column_for_byte_offset},
    },
};

use super::view::MarkdownEditor;

pub(super) fn build_document_input(
    text: &str,
    window: &mut Window,
    cx: &mut Context<InputState>,
) -> InputState {
    let mut state = InputState::new(window, cx)
        .auto_grow(1, 4096)
        .soft_wrap(true)
        .placeholder("Start writing...");
    state.set_value(text.to_string(), window, cx);
    state
}

impl MarkdownEditor {
    pub(super) fn sync_input_from_snapshot(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let snapshot = self.snapshot.clone();
        self.syncing_input = true;
        self.document_input.update(cx, |input, cx| {
            let current_text = input.text().to_string();
            if current_text != snapshot.display_map.visible_text {
                input.set_value(snapshot.display_map.visible_text.clone(), window, cx);
            }

            if input.cursor() != snapshot.visible_selection.cursor() {
                input.set_cursor_position(
                    Position {
                        line: snapshot.visible_caret_position.line as u32,
                        character: snapshot.visible_caret_position.column as u32,
                    },
                    window,
                    cx,
                );
            }
        });
        self.syncing_input = false;
    }

    pub(super) fn handle_input_event(
        &mut self,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.syncing_input {
            return;
        }

        match event {
            InputEvent::Change => self.sync_from_input(window, cx, true),
            InputEvent::Focus => {
                self.input_focused = true;
                self.sync_from_input(window, cx, false);
                cx.notify();
            }
            InputEvent::Blur => {
                self.input_focused = false;
                self.sync_from_input(window, cx, false);
                cx.notify();
            }
            InputEvent::PressEnter { .. } => {}
        }
    }

    pub(super) fn handle_observed_input_change(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.syncing_input {
            return;
        }

        self.sync_from_input(window, cx, false);
    }

    fn sync_from_input(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        allow_autosave: bool,
    ) {
        let Some((visible_text, visible_selection)) = self.read_input_state(window, cx) else {
            return;
        };

        let effects = if visible_text != self.snapshot.display_map.visible_text {
            let Some((text, selection)) =
                reconcile_visible_input_change(&self.snapshot, &visible_text)
            else {
                return;
            };
            self.controller
                .dispatch(EditCommand::SyncDocumentState { text, selection })
        } else if visible_selection != self.snapshot.visible_selection {
            let selection = selection_from_visible_input(&self.snapshot, &visible_selection);
            self.controller
                .dispatch(EditCommand::SetSelection { selection })
        } else {
            return;
        };

        if allow_autosave && effects.changed {
            self.schedule_autosave(window, cx);
        }
        self.apply_effects(window, cx, effects);
    }

    fn read_input_state(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<(String, SelectionState)> {
        self.document_input.update(cx, |input, cx| {
            if input.marked_text_range(window, cx).is_some() {
                return None;
            }

            let text = input.text().to_string();
            let cursor = input.cursor();
            let preferred_column = Some(input.cursor_position().character as usize);
            let selection = selection_from_input(
                &text,
                input
                    .selected_text_range(true, window, cx)
                    .map(|selection| selection.range),
                cursor,
                preferred_column,
            );
            Some((text, selection))
        })
    }

    pub(super) fn input_has_marked_text(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        self.document_input.update(cx, |input, cx| {
            input.marked_text_range(window, cx).is_some()
        })
    }
}

fn selection_from_visible_input(
    snapshot: &EditorSnapshot,
    visible_selection: &SelectionState,
) -> SelectionState {
    let visible_selection = normalize_vertical_gap_selection(snapshot, visible_selection);
    if !visible_selection.is_collapsed() {
        return snapshot
            .display_map
            .visible_selection_to_source(&visible_selection);
    }

    if let Some(span) =
        hidden_block_syntax_span_at_visible_cursor(snapshot, visible_selection.cursor())
    {
        let mut selection = SelectionState::collapsed(span.source_range.end);
        selection.preferred_column = visible_selection.preferred_column;
        selection.affinity =
            if should_reveal_hidden_block_syntax_boundary(snapshot, &visible_selection) {
                SelectionAffinity::Upstream
            } else {
                SelectionAffinity::Downstream
            };
        return selection;
    }

    let mut mapping_selection = visible_selection.clone();
    mapping_selection.affinity = SelectionAffinity::Downstream;
    let mut source_selection = snapshot
        .display_map
        .visible_selection_to_source(&mapping_selection);

    if should_reveal_hidden_block_syntax_boundary(snapshot, &visible_selection) {
        source_selection.affinity = SelectionAffinity::Upstream;
    }

    source_selection
}

fn normalize_vertical_gap_selection(
    snapshot: &EditorSnapshot,
    visible_selection: &SelectionState,
) -> SelectionState {
    if !visible_selection.is_collapsed() || !snapshot.visible_selection.is_collapsed() {
        return visible_selection.clone();
    }

    let visible_text = &snapshot.display_map.visible_text;
    let previous_cursor = clamp_to_char_boundary(visible_text, snapshot.visible_selection.cursor());
    let next_cursor = clamp_to_char_boundary(visible_text, visible_selection.cursor());
    if next_cursor == previous_cursor {
        return visible_selection.clone();
    }

    let (previous_line, _) = line_column_for_byte_offset(visible_text, previous_cursor);
    let (next_line, _) = line_column_for_byte_offset(visible_text, next_cursor);
    if previous_line == next_line {
        return visible_selection.clone();
    }

    let Some(adjusted_cursor) = adjusted_cursor_for_hidden_vertical_gap(
        &snapshot.display_map.blocks,
        previous_cursor,
        next_cursor,
    ) else {
        return visible_selection.clone();
    };

    let mut adjusted = visible_selection.clone();
    adjusted.anchor_byte = adjusted_cursor;
    adjusted.head_byte = adjusted_cursor;
    adjusted
}

fn adjusted_cursor_for_hidden_vertical_gap(
    blocks: &[crate::RenderBlock],
    previous_cursor: usize,
    next_cursor: usize,
) -> Option<usize> {
    let moving_down = next_cursor > previous_cursor;

    for window in blocks.windows(2) {
        let previous_block = &window[0];
        let next_block = &window[1];
        if block_has_rendered_text(previous_block) {
            continue;
        }

        let empty_block_start = previous_block.visible_range.start;
        let next_block_start = next_block.visible_range.start;
        if next_block_start <= empty_block_start {
            continue;
        }

        if moving_down
            && previous_cursor >= empty_block_start
            && previous_cursor < next_block_start
            && next_cursor >= empty_block_start
            && next_cursor < next_block_start
        {
            return Some(next_block_start);
        }

        if !moving_down
            && previous_cursor >= next_block_start
            && next_cursor >= empty_block_start
            && next_cursor < next_block_start
        {
            return Some(empty_block_start);
        }
    }

    None
}

fn block_has_rendered_text(block: &crate::RenderBlock) -> bool {
    block.spans.iter().any(|span| {
        !span.visible_text.is_empty()
            && !matches!(
                span.kind,
                RenderSpanKind::LineBreak | RenderSpanKind::HiddenSyntax
            )
    })
}

fn should_reveal_hidden_block_syntax_boundary(
    snapshot: &EditorSnapshot,
    visible_selection: &SelectionState,
) -> bool {
    if !visible_selection.is_collapsed() || !snapshot.visible_selection.is_collapsed() {
        return false;
    }

    let visible_text = &snapshot.display_map.visible_text;
    let previous_cursor = clamp_to_char_boundary(visible_text, snapshot.visible_selection.cursor());
    let next_cursor = clamp_to_char_boundary(visible_text, visible_selection.cursor());
    if next_cursor >= previous_cursor {
        return false;
    }

    let (previous_line, _) = line_column_for_byte_offset(visible_text, previous_cursor);
    let (next_line, next_column) = line_column_for_byte_offset(visible_text, next_cursor);
    if previous_line != next_line || next_column != 0 {
        return false;
    }

    hidden_block_syntax_span_at_visible_cursor(snapshot, next_cursor).is_some()
}

fn hidden_block_syntax_span_at_visible_cursor(
    snapshot: &EditorSnapshot,
    visible_cursor: usize,
) -> Option<&crate::RenderSpan> {
    snapshot.display_map.blocks.iter().find_map(|block| {
        if !matches!(
            block.kind,
            BlockKind::Heading { .. } | BlockKind::Blockquote | BlockKind::List
        ) {
            return None;
        }

        block.spans.iter().find(|span| {
            matches!(
                span.kind,
                RenderSpanKind::HiddenSyntax | RenderSpanKind::ListMarker
            ) && span.visible_range.start == visible_cursor
                && span.visible_range.is_empty()
        })
    })
}

pub(super) fn selection_from_input(
    text: &str,
    selection_utf16: Option<std::ops::Range<usize>>,
    cursor_byte: usize,
    preferred_column: Option<usize>,
) -> SelectionState {
    let cursor_byte = clamp_to_char_boundary(text, cursor_byte);
    let Some(selection_utf16) = selection_utf16 else {
        let mut selection = SelectionState::collapsed(cursor_byte);
        selection.preferred_column = preferred_column;
        return selection;
    };

    let range = utf16_range_to_byte_range(text, &selection_utf16);
    if range.is_empty() {
        let mut selection = SelectionState::collapsed(cursor_byte);
        selection.preferred_column = preferred_column;
        return selection;
    }

    SelectionState {
        anchor_byte: if cursor_byte == range.start {
            range.end
        } else {
            range.start
        },
        head_byte: cursor_byte,
        preferred_column,
        affinity: if cursor_byte == range.start {
            SelectionAffinity::Upstream
        } else {
            SelectionAffinity::Downstream
        },
    }
}

fn utf16_range_to_byte_range(text: &str, range: &std::ops::Range<usize>) -> std::ops::Range<usize> {
    utf16_offset_to_byte_offset(text, range.start)..utf16_offset_to_byte_offset(text, range.end)
}

fn utf16_offset_to_byte_offset(text: &str, target: usize) -> usize {
    if target == 0 {
        return 0;
    }

    let mut utf16_offset = 0usize;
    for (byte_offset, ch) in text.char_indices() {
        if utf16_offset >= target {
            return byte_offset;
        }
        utf16_offset += ch.len_utf16();
        if utf16_offset >= target {
            return byte_offset + ch.len_utf8();
        }
    }

    text.len()
}

pub(super) fn reconcile_visible_input_change(
    snapshot: &EditorSnapshot,
    visible_text: &str,
) -> Option<(String, SelectionState)> {
    let (visible_range, replacement) =
        compute_document_diff(&snapshot.display_map.visible_text, visible_text)?;
    let source_range = snapshot
        .display_map
        .visible_selection_to_source(&SelectionState {
            anchor_byte: visible_range.start,
            head_byte: visible_range.end,
            preferred_column: None,
            affinity: SelectionAffinity::Downstream,
        })
        .range();

    let mut source_text = snapshot.document_text.clone();
    source_text.replace_range(source_range.clone(), &replacement);

    Some((
        source_text,
        SelectionState::collapsed(source_range.start + replacement.len()),
    ))
}

#[cfg(test)]
mod tests {
    use crate::core::controller::{DocumentSource, EditorController, SyncPolicy};

    use super::*;

    #[test]
    fn same_line_move_to_hidden_blockquote_start_requests_marker_reveal() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "> Quote".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(7),
        });
        let snapshot = controller.snapshot();

        let mut visible_selection = SelectionState::collapsed(0);
        visible_selection.preferred_column = Some(0);
        let mapped = selection_from_visible_input(&snapshot, &visible_selection);

        assert_eq!(mapped.cursor(), 2);
        assert_eq!(mapped.affinity, SelectionAffinity::Upstream);
    }

    #[test]
    fn same_line_move_to_hidden_heading_start_requests_marker_reveal() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "# Title".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(7),
        });
        let snapshot = controller.snapshot();

        let mut visible_selection = SelectionState::collapsed(0);
        visible_selection.preferred_column = Some(0);
        let mapped = selection_from_visible_input(&snapshot, &visible_selection);

        assert_eq!(mapped.cursor(), 2);
        assert_eq!(mapped.affinity, SelectionAffinity::Upstream);
    }

    #[test]
    fn vertical_move_to_hidden_blockquote_start_keeps_source_after_marker() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Above\n\n> Quote".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(6),
        });
        let snapshot = controller.snapshot();
        let blockquote = snapshot
            .display_map
            .blocks
            .iter()
            .find(|block| matches!(block.kind, BlockKind::Blockquote))
            .expect("blockquote block");
        let marker = blockquote
            .spans
            .iter()
            .find(|span| {
                span.kind == RenderSpanKind::HiddenSyntax && span.source_text.starts_with('>')
            })
            .expect("blockquote marker");

        let mut visible_selection = SelectionState::collapsed(blockquote.visible_range.start);
        visible_selection.preferred_column = Some(0);
        let mapped = selection_from_visible_input(&snapshot, &visible_selection);

        assert_eq!(mapped.cursor(), marker.source_range.end);
        assert_eq!(mapped.affinity, SelectionAffinity::Downstream);
    }

    #[test]
    fn selection_from_input_tracks_utf16_ranges_in_mixed_text() {
        let text = "A🙂中";
        let selection = selection_from_input(text, Some(1..4), text.len(), Some(3));

        assert_eq!(selection.range(), 1..text.len());
        assert_eq!(selection.cursor(), text.len());
        assert_eq!(selection.preferred_column, Some(3));
    }

    #[test]
    fn reconcile_visible_input_change_preserves_hidden_markup_with_mixed_unicode() {
        let controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "A🙂**中**".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        let snapshot = controller.snapshot();

        let (source_text, selection) =
            reconcile_visible_input_change(&snapshot, "A🙂新").expect("change should map");

        assert_eq!(source_text, "A🙂**新**");
        assert_eq!(selection, SelectionState::collapsed(10));
    }
}
