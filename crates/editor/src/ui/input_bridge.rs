use gpui::{Context, EntityInputHandler as _, Window};
use gpui_component::input::{InputEvent, InputState, Position};

use crate::{
    BlockKind, EditCommand, RenderSpanKind, SelectionAffinity, SelectionState,
    core::{
        controller::EditorSnapshot,
        table::{TableModel, TableNavDirection},
        text_ops::{
            byte_offset_for_line_column, clamp_to_char_boundary, compute_document_diff,
            line_column_for_byte_offset, utf16_range_to_byte_range,
        },
    },
};

use super::{
    surface::{
        caret_visual_offset_for_block, rendered_empty_block_line_count, rendered_text_for_block,
        rendered_visible_end, rendered_visible_len, surface_empty_block_line_count,
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

    pub(super) fn sync_selection_from_input(
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

#[cfg(test)]
mod table_reconcile_tests {
    use crate::core::{
        controller::{DocumentSource, EditorController, SyncPolicy},
        table::{TableCellRef, TableModel},
    };

    use super::*;

    #[test]
    fn reconcile_table_visible_input_change_rewrites_markdown_rows() {
        let source = "| Name|Role |\n| :--- | --- |\n| Ada|Eng|";
        let controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: source.to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        let snapshot = controller.snapshot();
        let edited_visible = snapshot
            .display_map
            .visible_text
            .replacen("Eng ", "Team", 1);

        let (source_text, _) = reconcile_visible_input_change(&snapshot, &edited_visible)
            .expect("table edit should map");

        assert_eq!(
            source_text,
            "| Name | Role |\n| :--- | --- |\n| Ada | Team |"
        );
    }

    #[test]
    fn reconcile_table_visible_input_change_normalizes_newlines_inside_cell() {
        let source = "| Name | Role |\n| --- | --- |\n| Ada |  |";
        let controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: source.to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        let snapshot = controller.snapshot();
        let table_block = snapshot
            .display_map
            .blocks
            .iter()
            .find(|block| block.kind == BlockKind::Table)
            .expect("table block");
        let table = TableModel::parse(&snapshot.document_text[table_block.content_range.clone()]);
        let role_cell = table
            .cell_source_range(TableCellRef {
                visible_row: 1,
                column: 1,
            })
            .expect("role cell");
        let visible_start = snapshot
            .display_map
            .source_to_visible(table_block.content_range.start + role_cell.start);
        let mut edited_visible = snapshot.display_map.visible_text.clone();
        edited_visible.replace_range(visible_start..visible_start, "Lead\nOps");

        let (source_text, selection) = reconcile_visible_input_change(&snapshot, &edited_visible)
            .expect("table edit should map");

        assert_eq!(
            source_text,
            "| Name | Role |\n| --- | --- |\n| Ada | Lead Ops |"
        );
        let rebuilt = TableModel::parse("| Name | Role |\n| --- | --- |\n| Ada | Lead Ops |");
        let rebuilt_role = rebuilt
            .cell_source_range(TableCellRef {
                visible_row: 1,
                column: 1,
            })
            .expect("rebuilt role cell");
        assert_eq!(
            selection,
            SelectionState {
                anchor_byte: rebuilt_role.start + "Lead Ops".len(),
                head_byte: rebuilt_role.start + "Lead Ops".len(),
                preferred_column: None,
                affinity: SelectionAffinity::Upstream,
            }
        );
    }

    #[test]
    fn reconcile_table_visible_input_change_preserves_inline_markup_and_unicode() {
        let source = concat!(
            "| Label | Ref |\n",
            "| --- | --- |\n",
            "| **\u{65B0}** | [docs](https://example.com) `ok` \u{1F642} |"
        );
        let controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: source.to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        let snapshot = controller.snapshot();
        let edited_visible =
            snapshot
                .display_map
                .visible_text
                .replacen("\u{65B0}", "\u{66F4}\u{65B0}", 1);

        let (source_text, _) = reconcile_visible_input_change(&snapshot, &edited_visible)
            .expect("table edit should map");

        assert_eq!(
            source_text,
            concat!(
                "| Label | Ref |\n",
                "| --- | --- |\n",
                "| **\u{66F4}\u{65B0}** | [docs](https://example.com) `ok` \u{1F642} |"
            )
        );
    }

    #[test]
    fn reconcile_table_visible_input_change_keeps_typing_inside_last_cell() {
        let source = "| 1 | 2 | 3 |\n| --- | --- | --- |\n| 11 | 22 |   |";
        let controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: source.to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        let snapshot = controller.snapshot();
        let first_visible = snapshot
            .display_map
            .visible_text
            .replacen("22   ", "22   3", 1);
        let (first_source, first_selection) =
            reconcile_visible_input_change(&snapshot, &first_visible).expect("first table edit");

        assert_eq!(
            first_source,
            "| 1 | 2 | 3 |\n| --- | --- | --- |\n| 11 | 22 | 3 |"
        );

        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: first_source,
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: first_selection,
        });
        let snapshot = controller.snapshot();
        let second_visible = snapshot
            .display_map
            .visible_text
            .replacen("22   3", "22   33", 1);
        let (second_source, _) =
            reconcile_visible_input_change(&snapshot, &second_visible).expect("second table edit");

        assert_eq!(
            second_source,
            "| 1 | 2 | 3 |\n| --- | --- | --- |\n| 11 | 22 | 33 |"
        );
    }

    #[test]
    fn reconcile_table_visible_input_change_keeps_cursor_at_cell_end_boundary() {
        let source = "| Name | Role |\n| --- | --- |\n| Ada | Eng |";
        let controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: source.to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        let snapshot = controller.snapshot();
        let edited_visible = snapshot.display_map.visible_text.replacen("Ada", "Adam", 1);

        let (_, selection) = reconcile_visible_input_change(&snapshot, &edited_visible)
            .expect("table edit should map");

        assert_eq!(selection.affinity, SelectionAffinity::Upstream);
    }
}

fn selection_from_visible_input(
    snapshot: &EditorSnapshot,
    visible_selection: &SelectionState,
) -> SelectionState {
    let input_selection = visible_selection.clone();
    let visible_selection = input_selection_to_display_selection(snapshot, visible_selection);
    let vertical_move = vertical_move_context(snapshot, &input_selection);
    if !visible_selection.is_collapsed() {
        let mut selection = snapshot
            .display_map
            .visible_selection_to_source(&visible_selection);
        if let Some(vertical_move) = vertical_move {
            selection.preferred_column = Some(vertical_move.preferred_column);
        }
        return selection;
    }

    if let Some(selection) = selection_for_compressed_gap_stop(
        snapshot,
        &input_selection,
        &visible_selection,
        vertical_move,
    ) {
        return selection;
    }

    if let Some(selection) = selection_for_compressed_gap_block_content(
        snapshot,
        &input_selection,
        &visible_selection,
        vertical_move,
    ) {
        return selection;
    }

    if let Some(selection) = selection_for_table_horizontal_gap_navigation(
        snapshot,
        &input_selection,
        &visible_selection,
    ) {
        return selection;
    }

    if let Some(span) =
        hidden_block_syntax_span_at_visible_cursor(snapshot, visible_selection.cursor())
    {
        let mut selection = SelectionState::collapsed(span.source_range.end);
        selection.preferred_column = vertical_move
            .map(|vertical_move| Some(vertical_move.preferred_column))
            .unwrap_or(visible_selection.preferred_column);
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
    if let Some(vertical_move) = vertical_move {
        source_selection.preferred_column = Some(vertical_move.preferred_column);
    }

    if should_reveal_hidden_block_syntax_boundary(snapshot, &visible_selection) {
        source_selection.affinity = SelectionAffinity::Upstream;
    }

    source_selection
}

fn selection_for_table_horizontal_gap_navigation(
    snapshot: &EditorSnapshot,
    input_selection: &SelectionState,
    visible_selection: &SelectionState,
) -> Option<SelectionState> {
    if !visible_selection.is_collapsed() {
        return None;
    }

    let previous_cursor = mirrored_input_selection(snapshot).cursor();
    let next_cursor = visible_selection.cursor();
    if previous_cursor == next_cursor {
        return None;
    }

    let visible_text = &snapshot.display_map.visible_text;
    let (previous_line, _) = line_column_for_byte_offset(visible_text, previous_cursor);
    let (next_line, _) = line_column_for_byte_offset(visible_text, next_cursor);
    if previous_line != next_line {
        return None;
    }

    let direction = if next_cursor > previous_cursor {
        TableNavDirection::Forward
    } else {
        TableNavDirection::Backward
    };

    snapshot
        .display_map
        .blocks
        .iter()
        .filter(|block| {
            block.kind == BlockKind::Table
                && previous_cursor >= block.visible_range.start
                && previous_cursor <= rendered_visible_end(block)
                && next_cursor >= block.visible_range.start
                && next_cursor <= rendered_visible_end(block)
        })
        .find_map(|block| {
            let table = TableModel::parse(&snapshot.document_text[block.content_range.clone()]);
            selection_for_table_gap_span(
                block,
                &table,
                previous_cursor,
                next_cursor,
                direction,
                input_selection
                    .preferred_column
                    .or(visible_selection.preferred_column),
            )
        })
}

fn selection_for_table_gap_span(
    block: &crate::RenderBlock,
    table: &TableModel,
    previous_cursor: usize,
    next_cursor: usize,
    direction: TableNavDirection,
    preferred_column: Option<usize>,
) -> Option<SelectionState> {
    for span in block.spans.iter().filter(|span| {
        span.kind == RenderSpanKind::Text
            && span.source_range.is_empty()
            && !span.hidden
            && !span.visible_text.is_empty()
    }) {
        match direction {
            TableNavDirection::Forward => {
                if previous_cursor != span.visible_range.start
                    || next_cursor <= previous_cursor
                    || next_cursor > span.visible_range.end
                {
                    continue;
                }

                let current = table.cell_ref_for_source_offset(
                    span.source_range
                        .start
                        .saturating_sub(block.content_range.start),
                    SelectionAffinity::Upstream,
                )?;
                let target = table.next_cell_ref(current, TableNavDirection::Forward)?;
                let source_offset =
                    block.content_range.start + table.cell_source_range(target)?.start;
                let mut selection = SelectionState::collapsed(source_offset);
                selection.preferred_column = preferred_column;
                selection.affinity = SelectionAffinity::Upstream;
                return Some(selection);
            }
            TableNavDirection::Backward => {
                if previous_cursor != span.visible_range.end
                    || next_cursor >= previous_cursor
                    || next_cursor < span.visible_range.start
                {
                    continue;
                }

                let current = table.cell_ref_for_source_offset(
                    span.source_range
                        .start
                        .saturating_sub(block.content_range.start),
                    SelectionAffinity::Upstream,
                )?;
                let source_offset =
                    block.content_range.start + table.cell_source_range(current)?.end;
                let mut selection = SelectionState::collapsed(source_offset);
                selection.preferred_column = preferred_column;
                selection.affinity = SelectionAffinity::Upstream;
                return Some(selection);
            }
            TableNavDirection::Down => {}
        }
    }

    None
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
    let visible_cursor = snapshot.visible_selection.cursor();
    let source_cursor = snapshot.selection.cursor();

    for gap in compressed_gap_regions(snapshot) {
        let Some(block) = blocks.get(gap.block_index) else {
            continue;
        };
        if source_cursor_owned_by_block_for_input_cursor(block, source_cursor) {
            return Some(gap.input_stop_offset);
        }

        let Some(next_block) = blocks.get(gap.next_block_index) else {
            continue;
        };
        if visible_cursor == gap.next_block_display_start
            && source_cursor >= next_block.source_range.start
            && source_cursor <= next_block.source_range.end
        {
            return Some(gap.next_block_input_start);
        }
    }

    for (block_index, block) in blocks.iter().enumerate() {
        if let Some(local_cursor) =
            caret_visual_offset_for_block(blocks, block_index, visible_cursor)
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
    let Some(direction) = VerticalInputDirection::from_lines(previous_line, next_line) else {
        return canonical_display_offset_for_input_offset(snapshot, next_cursor);
    };
    let preferred_column = preferred_column_for_vertical_move(snapshot, previous_cursor);

    let Some(adjusted_cursor) = adjusted_cursor_for_hidden_vertical_gap(
        snapshot,
        previous_cursor,
        next_cursor,
        direction,
        preferred_column,
    ) else {
        let Some(adjusted_cursor) = adjusted_cursor_for_virtual_inter_block_gap(
            snapshot,
            previous_cursor,
            next_cursor,
            direction,
            preferred_column,
        ) else {
            return canonical_display_offset_for_input_offset(snapshot, next_cursor);
        };
        return adjusted_cursor;
    };
    adjusted_cursor
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VerticalInputDirection {
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VerticalMoveContext {
    previous_cursor: usize,
    next_cursor: usize,
    direction: VerticalInputDirection,
    preferred_column: usize,
}

impl VerticalInputDirection {
    fn from_lines(previous_line: usize, next_line: usize) -> Option<Self> {
        match next_line.cmp(&previous_line) {
            std::cmp::Ordering::Less => Some(Self::Up),
            std::cmp::Ordering::Greater => Some(Self::Down),
            std::cmp::Ordering::Equal => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompressedGapCursorState {
    BeforeGap,
    GapStop,
    AfterGap,
}

fn adjusted_cursor_for_hidden_vertical_gap(
    snapshot: &EditorSnapshot,
    previous_cursor: usize,
    next_cursor: usize,
    direction: VerticalInputDirection,
    preferred_column: usize,
) -> Option<usize> {
    for gap in compressed_gap_regions(snapshot) {
        let previous_state = gap.cursor_state(previous_cursor);
        let next_state = gap.cursor_state(next_cursor);
        match (direction, previous_state, next_state) {
            (
                VerticalInputDirection::Down,
                CompressedGapCursorState::BeforeGap,
                CompressedGapCursorState::GapStop,
            ) => return Some(gap.display_blank_offset),
            (
                VerticalInputDirection::Down,
                CompressedGapCursorState::BeforeGap,
                CompressedGapCursorState::AfterGap,
            ) => {
                return Some(gap.next_block_vertical_target(snapshot, direction, preferred_column));
            }
            (
                VerticalInputDirection::Down,
                CompressedGapCursorState::GapStop,
                CompressedGapCursorState::GapStop | CompressedGapCursorState::AfterGap,
            ) => {
                return Some(gap.next_block_vertical_target(snapshot, direction, preferred_column));
            }
            (
                VerticalInputDirection::Up,
                CompressedGapCursorState::AfterGap,
                CompressedGapCursorState::GapStop,
            ) => return Some(gap.display_blank_offset),
            (
                VerticalInputDirection::Up,
                CompressedGapCursorState::AfterGap,
                CompressedGapCursorState::BeforeGap,
            ) => {
                return Some(gap.previous_block_vertical_target(
                    snapshot,
                    direction,
                    preferred_column,
                ));
            }
            (
                VerticalInputDirection::Up,
                CompressedGapCursorState::GapStop,
                CompressedGapCursorState::BeforeGap | CompressedGapCursorState::GapStop,
            ) => {
                return Some(gap.previous_block_vertical_target(
                    snapshot,
                    direction,
                    preferred_column,
                ));
            }
            _ => {}
        }
    }

    None
}

fn adjusted_cursor_for_virtual_inter_block_gap(
    snapshot: &EditorSnapshot,
    previous_cursor: usize,
    next_cursor: usize,
    direction: VerticalInputDirection,
    preferred_column: usize,
) -> Option<usize> {
    let blocks = &snapshot.display_map.blocks;

    for (block_index, block) in blocks.iter().enumerate() {
        let Some(next_block) = blocks.get(block_index + 1) else {
            continue;
        };

        if rendered_visible_len(block) == 0 || rendered_visible_len(next_block) == 0 {
            continue;
        }

        let block_end = rendered_visible_end(block);
        let next_block_start = next_block.visible_range.start;
        if next_block_start <= block_end {
            continue;
        }

        match direction {
            VerticalInputDirection::Down => {
                let started_in_current_block =
                    caret_visual_offset_for_block(blocks, block_index, previous_cursor).is_some();
                let landed_in_virtual_gap =
                    next_cursor >= block_end && next_cursor < next_block_start;
                if started_in_current_block && landed_in_virtual_gap {
                    return Some(vertical_target_display_offset_for_block(
                        snapshot,
                        next_block,
                        direction,
                        preferred_column,
                    ));
                }
            }
            VerticalInputDirection::Up => {
                let started_in_next_block =
                    caret_visual_offset_for_block(blocks, block_index + 1, previous_cursor)
                        .is_some();
                let landed_in_virtual_gap =
                    next_cursor > block_end && next_cursor <= next_block_start;
                if started_in_next_block && landed_in_virtual_gap {
                    return Some(vertical_target_display_offset_for_block(
                        snapshot,
                        block,
                        direction,
                        preferred_column,
                    ));
                }
            }
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
        if input_offset == gap.input_stop_offset {
            return gap.display_blank_offset;
        }
        if input_offset > gap.input_stop_offset && input_offset < gap.next_block_input_start {
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
    block_index: usize,
    next_block_index: usize,
    input_stop_offset: usize,
    next_block_input_start: usize,
    display_blank_offset: usize,
    previous_display_end: usize,
    next_block_display_start: usize,
}

impl CompressedGapRegion {
    fn cursor_state(&self, cursor: usize) -> CompressedGapCursorState {
        if cursor < self.input_stop_offset {
            CompressedGapCursorState::BeforeGap
        } else if cursor < self.next_block_input_start {
            CompressedGapCursorState::GapStop
        } else {
            CompressedGapCursorState::AfterGap
        }
    }

    fn next_block_vertical_target(
        &self,
        snapshot: &EditorSnapshot,
        direction: VerticalInputDirection,
        preferred_column: usize,
    ) -> usize {
        let Some(block) = snapshot.display_map.blocks.get(self.next_block_index) else {
            return self.next_block_display_start;
        };

        vertical_target_display_offset_for_block(snapshot, block, direction, preferred_column)
    }

    fn previous_block_vertical_target(
        &self,
        snapshot: &EditorSnapshot,
        direction: VerticalInputDirection,
        preferred_column: usize,
    ) -> usize {
        let Some(block) = self
            .block_index
            .checked_sub(1)
            .and_then(|index| snapshot.display_map.blocks.get(index))
        else {
            return self.previous_display_end;
        };

        vertical_target_display_offset_for_block(snapshot, block, direction, preferred_column)
    }
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
        let input_stop_offset = rendered_visible_end(previous_block).saturating_add(1);
        if next_block.visible_range.start <= input_stop_offset {
            continue;
        }

        regions.push(CompressedGapRegion {
            block_index,
            next_block_index: block_index + 1,
            input_stop_offset,
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

fn content_display_start_for_block(snapshot: &EditorSnapshot, block: &crate::RenderBlock) -> usize {
    let _ = snapshot;
    block.visible_range.start
}

fn content_display_end_for_block(snapshot: &EditorSnapshot, block: &crate::RenderBlock) -> usize {
    let _ = snapshot;
    rendered_visible_end(block)
}

fn preferred_column_for_vertical_move(snapshot: &EditorSnapshot, previous_cursor: usize) -> usize {
    snapshot
        .selection
        .preferred_column
        .or(mirrored_input_selection(snapshot).preferred_column)
        .unwrap_or_else(|| {
            line_column_for_byte_offset(&snapshot.display_map.visible_text, previous_cursor).1
        })
}

fn vertical_move_context(
    snapshot: &EditorSnapshot,
    input_selection: &SelectionState,
) -> Option<VerticalMoveContext> {
    let visible_text = &snapshot.display_map.visible_text;
    let previous_cursor =
        clamp_to_char_boundary(visible_text, mirrored_input_selection(snapshot).cursor());
    let next_cursor = clamp_to_char_boundary(visible_text, input_selection.cursor());
    let direction = VerticalInputDirection::from_lines(
        line_column_for_byte_offset(visible_text, previous_cursor).0,
        line_column_for_byte_offset(visible_text, next_cursor).0,
    )?;

    Some(VerticalMoveContext {
        previous_cursor,
        next_cursor,
        direction,
        preferred_column: preferred_column_for_vertical_move(snapshot, previous_cursor),
    })
}

fn vertical_target_display_offset_for_block(
    snapshot: &EditorSnapshot,
    block: &crate::RenderBlock,
    direction: VerticalInputDirection,
    preferred_column: usize,
) -> usize {
    let rendered_text = rendered_text_for_block(block);
    if rendered_text.is_empty() {
        return match direction {
            VerticalInputDirection::Down => content_display_start_for_block(snapshot, block),
            VerticalInputDirection::Up => content_display_end_for_block(snapshot, block),
        };
    }

    let target_line = match direction {
        VerticalInputDirection::Down => 0,
        VerticalInputDirection::Up => rendered_text.lines().count().saturating_sub(1),
    };
    let local_offset = byte_offset_for_line_column(&rendered_text, target_line, preferred_column);
    clamp_to_char_boundary(
        &snapshot.display_map.visible_text,
        block.visible_range.start + local_offset,
    )
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

fn selection_for_compressed_gap_block_content(
    snapshot: &EditorSnapshot,
    _input_selection: &SelectionState,
    visible_selection: &SelectionState,
    vertical_move: Option<VerticalMoveContext>,
) -> Option<SelectionState> {
    if !visible_selection.is_collapsed() {
        return None;
    }

    let vertical_move = vertical_move?;
    let previous_cursor = vertical_move.previous_cursor;
    let next_cursor = vertical_move.next_cursor;
    let direction = vertical_move.direction;

    for gap in compressed_gap_regions(snapshot) {
        let previous_state = gap.cursor_state(previous_cursor);
        let next_state = gap.cursor_state(next_cursor);
        let target_block = match (direction, previous_state, next_state) {
            (
                VerticalInputDirection::Down,
                CompressedGapCursorState::BeforeGap | CompressedGapCursorState::GapStop,
                CompressedGapCursorState::GapStop | CompressedGapCursorState::AfterGap,
            ) => snapshot.display_map.blocks.get(gap.next_block_index),
            (
                VerticalInputDirection::Up,
                CompressedGapCursorState::AfterGap | CompressedGapCursorState::GapStop,
                CompressedGapCursorState::BeforeGap | CompressedGapCursorState::GapStop,
            ) => gap
                .block_index
                .checked_sub(1)
                .and_then(|index| snapshot.display_map.blocks.get(index)),
            _ => None,
        }?;

        if !matches!(
            target_block.kind,
            BlockKind::Heading { .. } | BlockKind::Blockquote | BlockKind::List
        ) {
            continue;
        }

        let expected_cursor = vertical_target_display_offset_for_block(
            snapshot,
            target_block,
            direction,
            vertical_move.preferred_column,
        );
        if visible_selection.cursor() != expected_cursor {
            continue;
        }

        let mut selection = SelectionState::collapsed(expected_cursor);
        selection.preferred_column = Some(vertical_move.preferred_column);
        selection.affinity = match direction {
            VerticalInputDirection::Down => SelectionAffinity::Downstream,
            VerticalInputDirection::Up => SelectionAffinity::Upstream,
        };
        return Some(snapshot.display_map.visible_selection_to_source(&selection));
    }

    None
}

fn selection_for_compressed_gap_stop(
    snapshot: &EditorSnapshot,
    _input_selection: &SelectionState,
    visible_selection: &SelectionState,
    vertical_move: Option<VerticalMoveContext>,
) -> Option<SelectionState> {
    if !visible_selection.is_collapsed() {
        return None;
    }

    let vertical_move = vertical_move?;
    let previous_cursor = vertical_move.previous_cursor;
    let next_cursor = vertical_move.next_cursor;
    let direction = vertical_move.direction;

    for gap in compressed_gap_regions(snapshot) {
        if next_cursor != gap.display_blank_offset {
            continue;
        }

        let previous_state = gap.cursor_state(previous_cursor);
        let next_state = gap.cursor_state(next_cursor);
        let is_gap_stop_move = matches!(
            (direction, previous_state, next_state),
            (
                VerticalInputDirection::Down,
                CompressedGapCursorState::BeforeGap,
                CompressedGapCursorState::GapStop
            ) | (
                VerticalInputDirection::Up,
                CompressedGapCursorState::AfterGap,
                CompressedGapCursorState::GapStop
            )
        );
        if !is_gap_stop_move {
            continue;
        }

        let block = snapshot.display_map.blocks.get(gap.block_index)?;
        let mut selection = SelectionState::collapsed(block.content_range.start);
        selection.preferred_column = Some(vertical_move.preferred_column);
        selection.affinity = SelectionAffinity::Downstream;
        return Some(selection);
    }

    None
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
    if let Some(mapped) =
        reconcile_table_visible_input_change(snapshot, &visible_range, &replacement)
    {
        return Some(mapped);
    }
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

fn reconcile_table_visible_input_change(
    snapshot: &EditorSnapshot,
    visible_range: &std::ops::Range<usize>,
    replacement: &str,
) -> Option<(String, SelectionState)> {
    let (_, block) = table_block_for_visible_range(snapshot, visible_range)?;
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
    if source_range.start < block.content_range.start || source_range.end > block.content_range.end
    {
        return None;
    }

    let table = TableModel::parse(
        &snapshot.document_text[block.content_range.start..block.content_range.end],
    );
    let local_range = normalize_table_local_range(
        snapshot,
        block,
        &table,
        &display_range,
        &source_range,
        replacement,
    );
    let start_cell =
        table.cell_ref_for_source_offset(local_range.start, SelectionAffinity::Downstream)?;
    let end_cell =
        table.cell_ref_for_source_offset(local_range.end, SelectionAffinity::Upstream)?;
    if start_cell != end_cell {
        return None;
    }

    let cell_range = table.cell_source_range(start_cell)?;
    if local_range.start < cell_range.start || local_range.end > cell_range.end {
        return None;
    }

    let normalized = normalize_table_cell_replacement(replacement);
    let relative_cell_range = local_range.start.saturating_sub(cell_range.start)
        ..local_range.end.saturating_sub(cell_range.start);
    let current_cell_source = table.cell_source_text(start_cell).unwrap_or("").to_string();
    let mut updated_cell_source = current_cell_source.clone();
    updated_cell_source.replace_range(relative_cell_range.clone(), &normalized);

    let replacement_table =
        table.rebuild_markdown_with_override(start_cell, updated_cell_source.clone());
    let mut source_text = snapshot.document_text.clone();
    source_text.replace_range(block.content_range.clone(), &replacement_table);

    let rebuilt = TableModel::parse(&replacement_table);
    let rebuilt_cell_range = rebuilt.cell_source_range(start_cell)?;
    let cursor_in_cell = relative_cell_range.start + normalized.len();
    let rebuilt_cursor = block.content_range.start
        + rebuilt_cell_range.start
        + escaped_table_cell_cursor_offset(&updated_cell_source, cursor_in_cell);
    let mut selection = SelectionState::collapsed(rebuilt_cursor);
    if cursor_in_cell >= updated_cell_source.len() {
        selection.affinity = SelectionAffinity::Upstream;
    }

    Some((source_text, selection))
}

fn table_block_for_visible_range<'a>(
    snapshot: &'a EditorSnapshot,
    visible_range: &std::ops::Range<usize>,
) -> Option<(usize, &'a crate::RenderBlock)> {
    snapshot
        .display_map
        .blocks
        .iter()
        .enumerate()
        .find(|(_, block)| {
            block.kind == BlockKind::Table
                && visible_range.start >= block.visible_range.start
                && visible_range.end <= rendered_visible_end(block)
        })
}

fn normalize_table_cell_replacement(replacement: &str) -> String {
    replacement.replace("\r\n", " ").replace(['\r', '\n'], " ")
}

fn normalize_table_local_range(
    snapshot: &EditorSnapshot,
    block: &crate::RenderBlock,
    table: &TableModel,
    display_range: &std::ops::Range<usize>,
    source_range: &std::ops::Range<usize>,
    replacement: &str,
) -> std::ops::Range<usize> {
    let mut local_range = source_range.start.saturating_sub(block.content_range.start)
        ..source_range.end.saturating_sub(block.content_range.start);
    if !display_range.is_empty() || replacement.is_empty() {
        return local_range;
    }

    let downstream_local = local_range.start;
    let upstream_local = snapshot
        .display_map
        .visible_to_source_with_affinity(display_range.start, SelectionAffinity::Upstream)
        .source_offset
        .saturating_sub(block.content_range.start);

    let Some(cell_ref) =
        table.cell_ref_for_source_offset(downstream_local, SelectionAffinity::Downstream)
    else {
        return local_range;
    };
    let Some(cell_range) = table.cell_source_range(cell_ref) else {
        return local_range;
    };

    if downstream_local >= cell_range.start && downstream_local <= cell_range.end {
        return local_range;
    }
    if upstream_local >= cell_range.start && upstream_local <= cell_range.end {
        local_range = upstream_local..upstream_local;
    }

    local_range
}

fn escaped_table_cell_cursor_offset(text: &str, raw_offset: usize) -> usize {
    let raw_offset = clamp_to_char_boundary(text, raw_offset);
    escape_table_cell_source(&text[..raw_offset]).len()
}

fn escape_table_cell_source(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    let mut previous_was_backslash = false;
    for ch in text.chars() {
        if ch == '|' && !previous_was_backslash {
            escaped.push('\\');
        }
        escaped.push(ch);
        previous_was_backslash = ch == '\\' && !previous_was_backslash;
    }
    escaped
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

        assert_eq!(mapped.cursor(), 5);
        assert_eq!(mapped.affinity, SelectionAffinity::Downstream);
    }

    #[test]
    fn collapsed_inter_block_gap_maps_second_down_press_to_hidden_heading_content_start() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "# A\n\n\n\n## AA".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(3),
        });
        let first_snapshot = controller.snapshot();
        let gap_cursor = mirrored_input_selection(&first_snapshot).cursor();
        let mut gap_selection = SelectionState::collapsed(gap_cursor);
        gap_selection.preferred_column = Some(0);
        let gap_source = selection_from_visible_input(&first_snapshot, &gap_selection);

        controller.dispatch(EditCommand::SetSelection {
            selection: gap_source,
        });
        let snapshot = controller.snapshot();
        let heading = snapshot
            .display_map
            .blocks
            .iter()
            .find(|block| matches!(block.kind, BlockKind::Heading { depth: 2 }))
            .expect("heading block");

        let mut visible_selection =
            SelectionState::collapsed(content_display_start_for_block(&snapshot, heading));
        visible_selection.preferred_column = Some(0);
        let mapped = selection_from_visible_input(&snapshot, &visible_selection);

        assert_eq!(
            mapped.cursor(),
            snapshot
                .display_map
                .visible_to_source_with_affinity(
                    heading.visible_range.start,
                    SelectionAffinity::Downstream,
                )
                .source_offset
        );
        assert_eq!(mapped.affinity, SelectionAffinity::Downstream);
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
