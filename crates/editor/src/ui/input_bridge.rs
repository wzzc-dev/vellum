use gpui::{Context, EntityInputHandler as _, Window};
use gpui_component::input::{InputEvent, InputState, Position};

use crate::{
    BlockKind, EditCommand, RenderSpanKind, SelectionAffinity, SelectionState,
    core::{
        controller::EditorSnapshot,
        text_ops::{
            clamp_to_char_boundary, compute_document_diff, line_column_for_byte_offset,
            utf16_range_to_byte_range,
        },
    },
};

use super::{
    surface::{
        caret_visual_offset_for_block, rendered_empty_block_line_count, rendered_visible_end,
        rendered_visible_len, surface_empty_block_line_count,
    },
    view::MarkdownEditor,
};

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

            if snapshot.visible_selection.is_collapsed() {
                let synced_selection = synced_input_selection(&snapshot);
                let has_range_selection = input
                    .selected_text_range(true, window, cx)
                    .map(|selection| !selection.range.is_empty())
                    .unwrap_or(false);
                if has_range_selection || input.cursor() != synced_selection.cursor() {
                    input.set_cursor_position(input_cursor_position(&snapshot), window, cx);
                }
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
        let mirrored_selection = mirrored_input_selection(&self.snapshot);

        let effects = if visible_text != self.snapshot.display_map.visible_text {
            let Some((text, selection)) =
                reconcile_visible_input_change(&self.snapshot, &visible_text)
            else {
                return;
            };
            self.controller
                .dispatch(EditCommand::SyncDocumentState { text, selection })
        } else if visible_selection != mirrored_selection {
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
    let visible_selection = input_selection_to_display_selection(snapshot, visible_selection);
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

fn input_cursor_position(snapshot: &EditorSnapshot) -> Position {
    let offset = input_cursor_offset(snapshot);
    let (line, column) = line_column_for_byte_offset(&snapshot.display_map.visible_text, offset);
    Position {
        line: line as u32,
        character: column as u32,
    }
}

fn synced_input_selection(snapshot: &EditorSnapshot) -> SelectionState {
    let offset = input_cursor_offset(snapshot);
    let (_, column) = line_column_for_byte_offset(&snapshot.display_map.visible_text, offset);
    let mut selection = SelectionState::collapsed(offset);
    selection.preferred_column = Some(column);
    selection
}

fn mirrored_input_selection(snapshot: &EditorSnapshot) -> SelectionState {
    if snapshot.visible_selection.is_collapsed() {
        synced_input_selection(snapshot)
    } else {
        snapshot.visible_selection.clone()
    }
}

fn input_cursor_offset(snapshot: &EditorSnapshot) -> usize {
    compressed_surface_cursor_offset(snapshot).unwrap_or(snapshot.visible_selection.cursor())
}

fn compressed_surface_cursor_offset(snapshot: &EditorSnapshot) -> Option<usize> {
    let blocks = &snapshot.display_map.blocks;
    let cursor = snapshot.visible_selection.cursor();
    let source_cursor = snapshot.selection.cursor();

    for (block_index, block) in blocks.iter().enumerate() {
        if let Some(local_cursor) = caret_visual_offset_for_block(blocks, block_index, cursor)
            && block_uses_compressed_surface_cursor(blocks, block_index)
            && source_cursor_owned_by_block_for_input_cursor(block, source_cursor)
        {
            let visible_offset =
                block.visible_range.start + local_cursor.min(rendered_visible_len(block));
            return Some(clamp_to_char_boundary(
                &snapshot.display_map.visible_text,
                visible_offset,
            ));
        }
    }

    None
}

fn source_cursor_owned_by_block_for_input_cursor(
    block: &crate::RenderBlock,
    source_cursor: usize,
) -> bool {
    source_cursor >= block.source_range.start && source_cursor < block.source_range.end
}

fn block_uses_compressed_surface_cursor(blocks: &[crate::RenderBlock], block_index: usize) -> bool {
    let Some(block) = blocks.get(block_index) else {
        return false;
    };

    surface_empty_block_line_count(blocks, block_index).is_some()
        || rendered_visible_len(block) == 0
}

fn input_selection_to_display_selection(
    snapshot: &EditorSnapshot,
    input_selection: &SelectionState,
) -> SelectionState {
    if !input_selection.is_collapsed() {
        let mut adjusted = input_selection.clone();
        adjusted.anchor_byte =
            canonical_display_offset_for_input_offset(snapshot, input_selection.anchor_byte);
        adjusted.head_byte =
            canonical_display_offset_for_input_offset(snapshot, input_selection.head_byte);
        return adjusted;
    }

    let mut adjusted = input_selection.clone();
    adjusted.anchor_byte = normalized_display_cursor_from_input(
        snapshot,
        mirrored_input_selection(snapshot).cursor(),
        input_selection.cursor(),
    );
    adjusted.head_byte = adjusted.anchor_byte;
    adjusted
}

fn normalized_display_cursor_from_input(
    snapshot: &EditorSnapshot,
    previous_input_cursor: usize,
    next_input_cursor: usize,
) -> usize {
    let visible_text = &snapshot.display_map.visible_text;
    let previous_cursor = clamp_to_char_boundary(visible_text, previous_input_cursor);
    let next_cursor = clamp_to_char_boundary(visible_text, next_input_cursor);
    if next_cursor == previous_cursor {
        return canonical_display_offset_for_input_offset(snapshot, next_cursor);
    }

    let (previous_line, _) = line_column_for_byte_offset(visible_text, previous_cursor);
    let (next_line, _) = line_column_for_byte_offset(visible_text, next_cursor);
    if previous_line == next_line {
        return canonical_display_offset_for_input_offset(snapshot, next_cursor);
    }

    let Some(adjusted_cursor) =
        adjusted_cursor_for_hidden_vertical_gap(snapshot, previous_cursor, next_cursor)
    else {
        return canonical_display_offset_for_input_offset(snapshot, next_cursor);
    };

    adjusted_cursor
}

fn adjusted_cursor_for_hidden_vertical_gap(
    snapshot: &EditorSnapshot,
    previous_cursor: usize,
    next_cursor: usize,
) -> Option<usize> {
    let moving_down = next_cursor > previous_cursor;

    for gap in compressed_gap_regions(snapshot) {
        if moving_down
            && previous_cursor < gap.input_blank_start
            && next_cursor >= gap.input_blank_start
            && next_cursor < gap.next_block_input_start
        {
            return Some(gap.display_blank_offset);
        }

        if moving_down
            && previous_cursor >= gap.input_blank_start
            && previous_cursor < gap.next_block_input_start
            && next_cursor >= gap.input_blank_start
            && next_cursor < gap.next_block_input_start
        {
            return Some(gap.next_block_display_start);
        }

        if !moving_down
            && previous_cursor >= gap.next_block_input_start
            && next_cursor >= gap.input_blank_start
            && next_cursor < gap.next_block_input_start
        {
            return Some(gap.display_blank_offset);
        }

        if !moving_down
            && previous_cursor >= gap.input_blank_start
            && previous_cursor < gap.next_block_input_start
            && next_cursor < gap.input_blank_start
        {
            return Some(gap.previous_display_end);
        }
    }

    None
}

fn canonical_display_offset_for_input_offset(
    snapshot: &EditorSnapshot,
    input_offset: usize,
) -> usize {
    let input_offset = clamp_to_char_boundary(&snapshot.display_map.visible_text, input_offset);
    for gap in compressed_gap_regions(snapshot) {
        if input_offset == gap.input_blank_start {
            return gap.display_blank_offset;
        }
        if input_offset > gap.input_blank_start && input_offset < gap.next_block_input_start {
            return gap.display_blank_offset;
        }
    }

    input_offset
}

fn block_has_compressed_vertical_gap(blocks: &[crate::RenderBlock], block_index: usize) -> bool {
    let Some(block) = blocks.get(block_index) else {
        return false;
    };
    if rendered_visible_len(block) == 0 {
        return true;
    }

    let Some(raw_line_count) = rendered_empty_block_line_count(block) else {
        return false;
    };
    let Some(surface_line_count) = surface_empty_block_line_count(blocks, block_index) else {
        return false;
    };

    surface_line_count < raw_line_count
}

#[derive(Debug, Clone, Copy)]
struct CompressedGapRegion {
    input_blank_start: usize,
    next_block_input_start: usize,
    display_blank_offset: usize,
    previous_display_end: usize,
    next_block_display_start: usize,
}

fn compressed_gap_regions(snapshot: &EditorSnapshot) -> Vec<CompressedGapRegion> {
    let blocks = &snapshot.display_map.blocks;
    let mut regions = Vec::new();

    for (block_index, block) in blocks.iter().enumerate() {
        if !block_has_compressed_vertical_gap(blocks, block_index) {
            continue;
        }

        let Some(previous_block) = block_index
            .checked_sub(1)
            .and_then(|index| blocks.get(index))
        else {
            continue;
        };
        let Some(next_block) = blocks.get(block_index + 1) else {
            continue;
        };
        let input_blank_start = rendered_visible_end(previous_block).saturating_add(1);
        if next_block.visible_range.start <= input_blank_start {
            continue;
        }

        regions.push(CompressedGapRegion {
            input_blank_start,
            next_block_input_start: next_block.visible_range.start,
            display_blank_offset: snapshot
                .display_map
                .source_to_visible(block.source_range.start),
            previous_display_end: rendered_visible_end(previous_block),
            next_block_display_start: snapshot
                .display_map
                .source_to_visible(next_block.source_range.start),
        });
    }

    regions
}

fn should_reveal_hidden_block_syntax_boundary(
    snapshot: &EditorSnapshot,
    visible_selection: &SelectionState,
) -> bool {
    if !visible_selection.is_collapsed() || !snapshot.visible_selection.is_collapsed() {
        return false;
    }

    let visible_text = &snapshot.display_map.visible_text;
    let previous_cursor =
        clamp_to_char_boundary(visible_text, mirrored_input_selection(snapshot).cursor());
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

pub(super) fn reconcile_visible_input_change(
    snapshot: &EditorSnapshot,
    visible_text: &str,
) -> Option<(String, SelectionState)> {
    let (visible_range, replacement) =
        compute_document_diff(&snapshot.display_map.visible_text, visible_text)?;
    let display_range = std::ops::Range {
        start: canonical_display_offset_for_input_offset(snapshot, visible_range.start),
        end: canonical_display_offset_for_input_offset(snapshot, visible_range.end),
    };
    let source_range = snapshot
        .display_map
        .visible_selection_to_source(&SelectionState {
            anchor_byte: display_range.start,
            head_byte: display_range.end,
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
    fn collapsed_inter_block_gap_maps_second_down_press_to_next_block() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "A\n\n\n\nB".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(3),
        });
        let snapshot = controller.snapshot();

        let mirrored_cursor = mirrored_input_selection(&snapshot).cursor();
        let mut visible_selection = SelectionState::collapsed(mirrored_cursor + 1);
        visible_selection.preferred_column = Some(0);
        let mapped = selection_from_visible_input(&snapshot, &visible_selection);

        assert_eq!(mirrored_cursor, 4);
        assert_eq!(mapped.cursor(), 5);
    }

    #[test]
    fn reconcile_visible_input_change_normalizes_typing_into_collapsed_gap() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "A\n\n\n\nB".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(3),
        });
        let snapshot = controller.snapshot();
        let mirrored_cursor = mirrored_input_selection(&snapshot).cursor();
        let mut visible_text = snapshot.display_map.visible_text.clone();
        visible_text.insert(mirrored_cursor, 'x');

        let (source_text, selection) =
            reconcile_visible_input_change(&snapshot, &visible_text).expect("change should map");

        assert_eq!(source_text, "A\n\nx\n\nB");
        assert_eq!(selection, SelectionState::collapsed(4));
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
