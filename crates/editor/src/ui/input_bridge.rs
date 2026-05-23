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

const AUTO_PAIR_OPENERS: &[(char, char)] =
    &[
        ('(', ')'),
        ('[', ']'),
        ('{', '}'),
        ('"', '"'),
        ('\'', '\''),
        ('`', '`'),
        ('$', '$'),
        ('<', '>'),
        ('*', '*'),
        ('_', '_'),
        ('~', '~'),
        ('=', '='),
        ('^', '^'),
    ];

fn closing_char_for_opener(c: char) -> Option<char> {
    AUTO_PAIR_OPENERS
        .iter()
        .find(|(open, _)| *open == c)
        .map(|(_, close)| *close)
}

fn is_auto_pair_closer(c: char) -> bool {
    AUTO_PAIR_OPENERS.iter().any(|(_, close)| *close == c)
}

fn detect_auto_pair_opportunity(
    old_visible: &str,
    new_visible: &str,
    old_selection: &SelectionState,
) -> Option<char> {
    if !old_selection.is_collapsed() {
        return None;
    }

    let (range, replacement) = compute_document_diff(old_visible, new_visible)?;
    if !range.is_empty() || replacement.chars().count() != 1 {
        return None;
    }

    let inserted = replacement.chars().next()?;
    closing_char_for_opener(inserted)
}

const MARKUP_WRAP_CHARS: &[(char, &str, &str)] = &[
    ('*', "**", "**"),
    ('_', "_", "_"),
    ('`', "`", "`"),
    ('~', "~~", "~~"),
    ('=', "==", "=="),
    ('^', "^", "^"),
    ('$', "$", "$"),
];

fn detect_wrap_selection_opportunity(
    old_visible: &str,
    new_visible: &str,
    old_selection: &SelectionState,
) -> Option<(&'static str, &'static str)> {
    if old_selection.is_collapsed() {
        return None;
    }

    let (range, replacement) = compute_document_diff(old_visible, new_visible)?;
    if !range.is_empty() || replacement.chars().count() != 1 {
        return None;
    }

    let inserted = replacement.chars().next()?;
    if let Some((before, after)) = wrap_pair_for_opener(inserted) {
        return Some((before, after));
    }
    if range.start == old_selection.range().end {
        if let Some((before, after)) = wrap_pair_for_closer(inserted) {
            return Some((before, after));
        }
    }

    MARKUP_WRAP_CHARS
        .iter()
        .find(|(c, _, _)| *c == inserted)
        .map(|(_, before, after)| (*before, *after))
}

fn wrap_pair_for_opener(c: char) -> Option<(&'static str, &'static str)> {
    match c {
        '(' => Some(("(", ")")),
        '[' => Some(("[", "]")),
        '{' => Some(("{", "}")),
        '"' => Some(("\"", "\"")),
        '\'' => Some(("'", "'")),
        '<' => Some(("<", ">")),
        _ => None,
    }
}

fn wrap_pair_for_closer(c: char) -> Option<(&'static str, &'static str)> {
    match c {
        ')' => Some(("(", ")")),
        ']' => Some(("[", "]")),
        '}' => Some(("{", "}")),
        '>' => Some(("<", ">")),
        _ => None,
    }
}

fn detect_link_paste_opportunity(
    old_visible: &str,
    new_visible: &str,
    old_selection: &SelectionState,
) -> Option<String> {
    if old_selection.is_collapsed() {
        return None;
    }

    let (range, replacement) = compute_document_diff(old_visible, new_visible)?;
    if range != old_selection.range() {
        return None;
    }

    let Some(destination) = normalized_pasted_url(&replacement) else {
        return None;
    };

    Some(destination.to_string())
}

fn detect_image_paste_opportunity(
    old_visible: &str,
    new_visible: &str,
    old_selection: &SelectionState,
) -> Option<String> {
    if old_selection.is_collapsed() {
        return None;
    }

    let (range, replacement) = compute_document_diff(old_visible, new_visible)?;
    if range != old_selection.range() {
        return None;
    }

    let Some(destination) = normalized_pasted_image_path(&replacement) else {
        return None;
    };

    Some(destination.to_string())
}

fn is_url_like(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let has_supported_scheme = lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("mailto:")
        || lower.starts_with("file://");
    has_supported_scheme
        && text
            .chars()
            .all(|ch| !ch.is_whitespace() && !ch.is_control())
}

fn normalized_pasted_url(text: &str) -> Option<&str> {
    if text.trim() != text {
        return None;
    }

    if is_url_like(text) {
        return Some(text);
    }

    let autolink = text.strip_prefix('<')?.strip_suffix('>')?;
    is_url_like(autolink).then_some(autolink)
}

fn normalized_pasted_image_path(text: &str) -> Option<&str> {
    if looks_like_image_path(text) {
        return Some(text);
    }

    let autolink = normalized_pasted_url(text)?;
    looks_like_image_path(autolink).then_some(autolink)
}

fn markdown_link(label: &str, destination: &str) -> String {
    format!(
        "[{}]({})",
        escape_link_label(label),
        escape_link_destination(destination)
    )
}

fn markdown_image(alt: &str, destination: &str) -> String {
    format!(
        "![{}]({})",
        escape_link_label(alt),
        escape_link_destination(destination)
    )
}

fn replace_selected_link_destination(
    snapshot: &EditorSnapshot,
    destination: &str,
) -> Option<(String, SelectionState)> {
    let selected_label_range = snapshot.selection.range();
    if selected_label_range.is_empty() {
        return None;
    }

    let link_span = snapshot
        .display_map
        .blocks
        .iter()
        .flat_map(|block| block.spans.iter())
        .find(|span| {
            matches!(span.meta, Some(crate::RenderSpanMeta::Link { .. }))
                && span.source_range.start <= selected_label_range.start
                && selected_label_range.end <= span.source_range.end
        })?;

    let label_end = link_span.source_range.end;
    let text = &snapshot.document_text;
    let target_start = label_end.checked_add(2)?;
    if text.as_bytes().get(link_span.source_range.start.checked_sub(1)?) != Some(&b'[')
        || text.as_bytes().get(label_end) != Some(&b']')
        || text.as_bytes().get(label_end + 1) != Some(&b'(')
        || target_start > text.len()
    {
        return None;
    }

    let target_len = link_target_source_len(&text[target_start..])?;
    let target_end = target_start.checked_add(target_len)?;
    if target_end > text.len() {
        return None;
    }

    let escaped_destination = escape_link_destination(destination);
    let mut updated = String::with_capacity(text.len() + escaped_destination.len() - target_len);
    updated.push_str(&text[..target_start]);
    updated.push_str(&escaped_destination);
    updated.push_str(&text[target_end..]);

    let cursor_after_destination = target_start + escaped_destination.len();
    let selection = if updated[cursor_after_destination..].starts_with(char::is_whitespace) {
        SelectionState::collapsed(cursor_after_destination)
    } else {
        SelectionState::collapsed(cursor_after_destination + 1)
    };
    Some((updated, selection))
}

fn replace_selected_image_source(
    snapshot: &EditorSnapshot,
    destination: &str,
) -> Option<(String, SelectionState)> {
    let selected_alt_range = snapshot.selection.range();
    if selected_alt_range.is_empty() {
        return None;
    }

    let image_span = snapshot
        .display_map
        .blocks
        .iter()
        .flat_map(|block| block.spans.iter())
        .find(|span| {
            let Some(alt_end) = image_alt_source_end(&snapshot.document_text, span) else {
                return false;
            };
            matches!(span.meta, Some(crate::RenderSpanMeta::Image { .. }))
                && span.source_range.start + 2 <= selected_alt_range.start
                && selected_alt_range.end <= alt_end
        })?;

    let alt_end = image_alt_source_end(&snapshot.document_text, image_span)?;
    let text = &snapshot.document_text;
    let source_start = alt_end.checked_add(2)?;
    if text.as_bytes().get(image_span.source_range.start) != Some(&b'!')
        || text.as_bytes().get(image_span.source_range.start + 1) != Some(&b'[')
        || text.as_bytes().get(alt_end) != Some(&b']')
        || text.as_bytes().get(alt_end + 1) != Some(&b'(')
        || source_start > text.len()
    {
        return None;
    }

    let source_len = link_target_source_len(&text[source_start..])?;
    let source_end = source_start.checked_add(source_len)?;
    if source_end > text.len() {
        return None;
    }

    let escaped_destination = escape_link_destination(destination);
    let mut updated = String::with_capacity(text.len() + escaped_destination.len() - source_len);
    updated.push_str(&text[..source_start]);
    updated.push_str(&escaped_destination);
    updated.push_str(&text[source_end..]);

    let cursor_after_source = source_start + escaped_destination.len();
    let selection = if updated[cursor_after_source..].starts_with(char::is_whitespace) {
        SelectionState::collapsed(cursor_after_source)
    } else {
        SelectionState::collapsed(cursor_after_source + 1)
    };
    Some((updated, selection))
}

fn image_alt_source_end(text: &str, image_span: &crate::RenderSpan) -> Option<usize> {
    let alt_start = image_span.source_range.start.checked_add(2)?;
    let rest = &text[alt_start..];
    rest
        .char_indices()
        .find_map(|(index, ch)| (ch == ']' && !is_escaped_byte(rest, index)).then_some(index))
        .map(|close| alt_start + close)
        .filter(|alt_end| *alt_end <= image_span.source_range.end)
}

fn link_target_source_len(rest: &str) -> Option<usize> {
    if let Some(after_open) = rest.strip_prefix('<') {
        return after_open.char_indices().find_map(|(index, ch)| {
            (ch == '>' && !is_escaped_byte(after_open, index)).then_some(index + 2)
        });
    }

    rest.char_indices()
        .find_map(|(index, ch)| {
            ((ch == ')' && !is_escaped_byte(rest, index)) || ch.is_whitespace()).then_some(index)
        })
        .filter(|len| *len > 0)
}

fn is_escaped_byte(text: &str, index: usize) -> bool {
    let bytes = text.as_bytes();
    let mut slash_count = 0usize;
    let mut cursor = index;
    while cursor > 0 && bytes[cursor - 1] == b'\\' {
        slash_count += 1;
        cursor -= 1;
    }
    slash_count % 2 == 1
}

fn looks_like_image_path(text: &str) -> bool {
    if text.trim() != text || text.chars().any(char::is_control) {
        return false;
    }

    let candidate = text
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(text)
        .split(['?', '#'])
        .next()
        .unwrap_or(text);
    let Some((_, ext)) = candidate.rsplit_once('.') else {
        return false;
    };

    matches!(
        ext.to_ascii_lowercase().as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" | "ico" | "tiff" | "tif"
    )
}

fn escape_link_label(text: &str) -> String {
    text.replace('\\', r"\\").replace(']', r"\]")
}

fn escape_link_destination(text: &str) -> String {
    text.replace('\\', r"\\").replace(')', r"\)")
}

fn detect_overclose_opportunity(
    old_visible: &str,
    new_visible: &str,
    old_selection: &SelectionState,
) -> bool {
    if !old_selection.is_collapsed() {
        return false;
    }

    let Some((range, replacement)) = compute_document_diff(old_visible, new_visible) else {
        return false;
    };
    if !range.is_empty() || replacement.chars().count() != 1 {
        return false;
    }

    let Some(typed) = replacement.chars().next() else {
        return false;
    };
    if !is_auto_pair_closer(typed) {
        return false;
    }

    let cursor = old_selection.cursor();
    let char_after = old_visible[cursor..].chars().next();
    char_after == Some(typed)
}

fn apply_auto_pair_to_source(
    source_text: &str,
    selection: SelectionState,
    close_char: char,
) -> (String, SelectionState) {
    if !selection.is_collapsed() {
        return (source_text.to_string(), selection);
    }

    let cursor = selection.cursor();
    let mut new_text = String::with_capacity(source_text.len() + close_char.len_utf8());
    new_text.push_str(&source_text[..cursor]);
    new_text.push(close_char);
    new_text.push_str(&source_text[cursor..]);

    (new_text, selection)
}

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
            let wrap_opportunity = detect_wrap_selection_opportunity(
                &self.snapshot.display_map.visible_text,
                &visible_text,
                &mirrored_selection,
            );
            let link_paste_opportunity = detect_link_paste_opportunity(
                &self.snapshot.display_map.visible_text,
                &visible_text,
                &mirrored_selection,
            );
            let image_paste_opportunity = detect_image_paste_opportunity(
                &self.snapshot.display_map.visible_text,
                &visible_text,
                &mirrored_selection,
            );

            if let Some(path) = image_paste_opportunity {
                self.syncing_input = true;
                self.document_input.update(cx, |input, cx| {
                    input.set_value(self.snapshot.display_map.visible_text.clone(), window, cx);
                });
                self.syncing_input = false;

                if let Some((text, selection)) = replace_selected_image_source(
                    &self.snapshot,
                    &path,
                ) {
                    self.controller
                        .dispatch(EditCommand::SyncDocumentState { text, selection })
                } else {
                    let selected_text = self
                        .snapshot
                        .document_text
                        .get(self.snapshot.selection.range())
                        .unwrap_or("");
                    let replacement = markdown_image(selected_text, &path);
                    self.controller
                        .dispatch(EditCommand::ReplaceSelection { text: replacement })
                }
            } else if let Some(url) = link_paste_opportunity {
                self.syncing_input = true;
                self.document_input.update(cx, |input, cx| {
                    input.set_value(self.snapshot.display_map.visible_text.clone(), window, cx);
                });
                self.syncing_input = false;

                if let Some((text, selection)) =
                    replace_selected_link_destination(&self.snapshot, &url)
                {
                    self.controller
                        .dispatch(EditCommand::SyncDocumentState { text, selection })
                } else {
                    let selected_text = self
                        .snapshot
                        .document_text
                        .get(self.snapshot.selection.range())
                        .unwrap_or("");
                    let replacement = markdown_link(selected_text, &url);
                    self.controller
                        .dispatch(EditCommand::ReplaceSelection { text: replacement })
                }
            } else if let Some((before, after)) = wrap_opportunity {
                self.syncing_input = true;
                self.document_input.update(cx, |input, cx| {
                    input.set_value(self.snapshot.display_map.visible_text.clone(), window, cx);
                });
                self.syncing_input = false;

                self.controller.dispatch(EditCommand::ToggleInlineMarkup {
                    before: before.to_string(),
                    after: after.to_string(),
                })
            } else {
                let overclose = detect_overclose_opportunity(
                    &self.snapshot.display_map.visible_text,
                    &visible_text,
                    &mirrored_selection,
                );

                if overclose {
                    let cursor = mirrored_selection.cursor();
                    let char_len = self.snapshot.display_map.visible_text[cursor..]
                        .chars()
                        .next()
                        .map(|c| c.len_utf8())
                        .unwrap_or(0);
                    let new_cursor = cursor + char_len;

                    self.syncing_input = true;
                    self.document_input.update(cx, |input, cx| {
                        input.set_value(self.snapshot.display_map.visible_text.clone(), window, cx);
                    });
                    self.syncing_input = false;

                    let mut new_visible_selection = SelectionState::collapsed(new_cursor);
                    new_visible_selection.preferred_column = visible_selection.preferred_column;
                    let selection =
                        selection_from_visible_input(&self.snapshot, &new_visible_selection);

                    self.controller
                        .dispatch(EditCommand::SetSelection { selection })
                } else {
                    let auto_pair_close = detect_auto_pair_opportunity(
                        &self.snapshot.display_map.visible_text,
                        &visible_text,
                        &mirrored_selection,
                    );

                    let Some((text, selection)) =
                        reconcile_visible_input_change(&self.snapshot, &visible_text)
                    else {
                        return;
                    };

                    let (text, selection) = if let Some(close_char) = auto_pair_close {
                        apply_auto_pair_to_source(&text, selection, close_char)
                    } else {
                        (text, selection)
                    };

                    self.controller
                        .dispatch(EditCommand::SyncDocumentState { text, selection })
                }
            }
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
    fn reconcile_table_visible_input_change_escapes_typed_pipe_in_cell() {
        let source = "| Name | Role |\n| --- | --- |\n| Ada | Lead |";
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
        let insert_at = table_block.content_range.start + role_cell.start + "Lead".len();
        let visible_insert_at = snapshot.display_map.source_to_visible(insert_at);
        let mut edited_visible = snapshot.display_map.visible_text.clone();
        edited_visible.replace_range(visible_insert_at..visible_insert_at, " | Ops");

        let (source_text, selection) = reconcile_visible_input_change(&snapshot, &edited_visible)
            .expect("table edit should map");

        assert_eq!(
            source_text,
            "| Name | Role |\n| --- | --- |\n| Ada | Lead \\| Ops |"
        );
        let rebuilt = TableModel::parse("| Name | Role |\n| --- | --- |\n| Ada | Lead \\| Ops |");
        let rebuilt_role = rebuilt
            .cell_source_range(TableCellRef {
                visible_row: 1,
                column: 1,
            })
            .expect("rebuilt role cell");
        assert_eq!(selection.cursor(), rebuilt_role.start + "Lead \\| Ops".len());
        assert_eq!(selection.affinity, SelectionAffinity::Upstream);
    }

    #[test]
    fn reconcile_table_visible_input_change_preserves_typed_backslash_before_pipe() {
        let source = "| Name | Role |\n| --- | --- |\n| Ada | Lead |";
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
        let insert_at = table_block.content_range.start + role_cell.start + "Lead".len();
        let visible_insert_at = snapshot.display_map.source_to_visible(insert_at);
        let mut edited_visible = snapshot.display_map.visible_text.clone();
        edited_visible.replace_range(visible_insert_at..visible_insert_at, r" \| Ops");

        let (source_text, selection) = reconcile_visible_input_change(&snapshot, &edited_visible)
            .expect("table edit should map");

        assert_eq!(
            source_text,
            r"| Name | Role |
| --- | --- |
| Ada | Lead \\\| Ops |"
        );

        let controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: source_text.clone(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        let snapshot = controller.snapshot();
        assert!(
            snapshot.display_map.visible_text.contains(r"Lead \| Ops"),
            "visible table text should preserve both the backslash and pipe"
        );
        let rebuilt = TableModel::parse(r"| Name | Role |
| --- | --- |
| Ada | Lead \\\| Ops |");
        let rebuilt_role = rebuilt
            .cell_source_range(TableCellRef {
                visible_row: 1,
                column: 1,
            })
            .expect("rebuilt role cell");
        assert_eq!(selection.cursor(), rebuilt_role.start + r"Lead \\\| Ops".len());
        assert_eq!(selection.affinity, SelectionAffinity::Upstream);
    }

    #[test]
    fn reconcile_table_visible_input_change_preserves_typed_backslash() {
        let source = "| Name | Path |\n| --- | --- |\n| Ada | C: |";
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
        let edited_visible = snapshot.display_map.visible_text.replacen("C:", r"C:\tmp", 1);

        let (source_text, _) = reconcile_visible_input_change(&snapshot, &edited_visible)
            .expect("table edit should map");

        assert_eq!(
            source_text,
            r"| Name | Path |
| --- | --- |
| Ada | C:\\tmp |"
        );

        let controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: source_text,
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        let snapshot = controller.snapshot();
        assert!(snapshot.display_map.visible_text.contains(r"C:\tmp"));
    }

    #[test]
    fn reconcile_table_visible_input_change_preserves_existing_escaped_pipe() {
        let source = "| Name | Role |\n| --- | --- |\n| Ada | Lead \\| Ops |";
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
        let edited_visible = snapshot.display_map.visible_text.replacen("Ops", "Ops!", 1);

        let (source_text, selection) = reconcile_visible_input_change(&snapshot, &edited_visible)
            .expect("table edit should map");

        assert_eq!(
            source_text,
            "| Name | Role |\n| --- | --- |\n| Ada | Lead \\| Ops! |"
        );
        let rebuilt = TableModel::parse("| Name | Role |\n| --- | --- |\n| Ada | Lead \\| Ops! |");
        let rebuilt_role = rebuilt
            .cell_source_range(TableCellRef {
                visible_row: 1,
                column: 1,
            })
            .expect("rebuilt role cell");
        assert_eq!(selection.cursor(), rebuilt_role.start + "Lead \\| Ops!".len());
        assert_eq!(selection.affinity, SelectionAffinity::Upstream);
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
    if block.embedded.is_some() {
        return false;
    }

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
    if block.embedded.is_some() {
        return false;
    }
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
            BlockKind::Heading { .. }
                | BlockKind::Blockquote
                | BlockKind::List
                | BlockKind::SourceCode
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
    let single_line = replacement.replace("\r\n", " ").replace(['\r', '\n'], " ");
    escape_visible_table_cell_text(&single_line)
}

fn escape_visible_table_cell_text(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' => escaped.push_str(r"\\"),
            '|' => escaped.push_str(r"\|"),
            _ => escaped.push(ch),
        }
    }
    escaped
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
    fn detects_url_paste_over_selection() {
        let old_visible = "Read docs";
        let new_visible = "Read https://example.com";
        let selection = SelectionState {
            anchor_byte: 5,
            head_byte: 9,
            preferred_column: None,
            affinity: SelectionAffinity::Downstream,
        };

        assert_eq!(
            detect_link_paste_opportunity(old_visible, new_visible, &selection),
            Some("https://example.com".to_string())
        );
    }

    #[test]
    fn detects_url_paste_with_case_insensitive_scheme() {
        let old_visible = "Read docs";
        let new_visible = "Read HTTPS://example.com";
        let selection = SelectionState {
            anchor_byte: 5,
            head_byte: 9,
            preferred_column: None,
            affinity: SelectionAffinity::Downstream,
        };

        assert_eq!(
            detect_link_paste_opportunity(old_visible, new_visible, &selection),
            Some("HTTPS://example.com".to_string())
        );
    }

    #[test]
    fn detects_file_url_paste_over_selection() {
        let old_visible = "Open report";
        let new_visible = "Open file:///tmp/report.md";
        let selection = SelectionState {
            anchor_byte: 5,
            head_byte: 11,
            preferred_column: None,
            affinity: SelectionAffinity::Downstream,
        };

        assert_eq!(
            detect_link_paste_opportunity(old_visible, new_visible, &selection),
            Some("file:///tmp/report.md".to_string())
        );
    }

    #[test]
    fn detects_markdown_autolink_paste_over_selection() {
        let old_visible = "Read docs";
        let new_visible = "Read <https://example.com/guide>";
        let selection = SelectionState {
            anchor_byte: 5,
            head_byte: 9,
            preferred_column: None,
            affinity: SelectionAffinity::Downstream,
        };

        assert_eq!(
            detect_link_paste_opportunity(old_visible, new_visible, &selection),
            Some("https://example.com/guide".to_string())
        );
    }

    #[test]
    fn replaces_selected_link_destination_when_pasting_url_over_label() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Read [docs](https://old.example)".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState {
                anchor_byte: "Read [".len(),
                head_byte: "Read [docs".len(),
                preferred_column: None,
                affinity: SelectionAffinity::Downstream,
            },
        });

        let snapshot = controller.snapshot();
        let (updated, selection) = replace_selected_link_destination(
            &snapshot,
            "https://new.example/guide",
        )
        .expect("selected link label should update destination");

        assert_eq!(updated, "Read [docs](https://new.example/guide)");
        assert_eq!(selection.cursor(), updated.len());
    }

    #[test]
    fn replacing_link_destination_preserves_title() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Read [docs](https://old.example \"Docs\")".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState {
                anchor_byte: "Read [".len(),
                head_byte: "Read [docs".len(),
                preferred_column: None,
                affinity: SelectionAffinity::Downstream,
            },
        });

        let snapshot = controller.snapshot();
        let (updated, selection) =
            replace_selected_link_destination(&snapshot, "https://new.example/guide")
                .expect("selected link label should update destination");

        assert_eq!(updated, "Read [docs](https://new.example/guide \"Docs\")");
        assert_eq!(selection.cursor(), "Read [docs](https://new.example/guide".len());
    }

    #[test]
    fn replacing_link_destination_handles_escaped_closing_parens() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Read [docs](https://old.example/a\\)b \"Docs\")".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState {
                anchor_byte: "Read [".len(),
                head_byte: "Read [docs".len(),
                preferred_column: None,
                affinity: SelectionAffinity::Downstream,
            },
        });

        let snapshot = controller.snapshot();
        let (updated, selection) =
            replace_selected_link_destination(&snapshot, "https://new.example/guide")
                .expect("selected link label should update destination");

        assert_eq!(updated, "Read [docs](https://new.example/guide \"Docs\")");
        assert_eq!(selection.cursor(), "Read [docs](https://new.example/guide".len());
    }

    #[test]
    fn replacing_link_destination_handles_escaped_angle_target_close() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Read [docs](<https://old.example/a\\>b> \"Docs\")".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState {
                anchor_byte: "Read [".len(),
                head_byte: "Read [docs".len(),
                preferred_column: None,
                affinity: SelectionAffinity::Downstream,
            },
        });

        let snapshot = controller.snapshot();
        let (updated, selection) =
            replace_selected_link_destination(&snapshot, "https://new.example/guide")
                .expect("selected link label should update destination");

        assert_eq!(updated, "Read [docs](https://new.example/guide \"Docs\")");
        assert_eq!(selection.cursor(), "Read [docs](https://new.example/guide".len());
    }

    #[test]
    fn replaces_selected_image_source_when_pasting_image_path_over_alt() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "See ![diagram](./assets/old.png)".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState {
                anchor_byte: "See ![".len(),
                head_byte: "See ![diagram".len(),
                preferred_column: None,
                affinity: SelectionAffinity::Downstream,
            },
        });

        let snapshot = controller.snapshot();
        let (updated, selection) =
            replace_selected_image_source(&snapshot, "./assets/new image(1).png")
                .expect("selected image alt should update source");

        assert_eq!(updated, r"See ![diagram](./assets/new image(1\).png)");
        assert_eq!(selection.cursor(), updated.len());
    }

    #[test]
    fn replacing_image_source_preserves_title() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "See ![diagram](./assets/old.png \"Diagram\")".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState {
                anchor_byte: "See ![".len(),
                head_byte: "See ![diagram".len(),
                preferred_column: None,
                affinity: SelectionAffinity::Downstream,
            },
        });

        let snapshot = controller.snapshot();
        let (updated, selection) =
            replace_selected_image_source(&snapshot, "./assets/new.png")
                .expect("selected image alt should update source");

        assert_eq!(updated, "See ![diagram](./assets/new.png \"Diagram\")");
        assert_eq!(selection.cursor(), "See ![diagram](./assets/new.png".len());
    }

    #[test]
    fn replacing_image_source_handles_escaped_closing_parens() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "See ![diagram](./assets/old\\)name.png \"Diagram\")".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState {
                anchor_byte: "See ![".len(),
                head_byte: "See ![diagram".len(),
                preferred_column: None,
                affinity: SelectionAffinity::Downstream,
            },
        });

        let snapshot = controller.snapshot();
        let (updated, selection) =
            replace_selected_image_source(&snapshot, "./assets/new.png")
                .expect("selected image alt should update source");

        assert_eq!(updated, "See ![diagram](./assets/new.png \"Diagram\")");
        assert_eq!(selection.cursor(), "See ![diagram](./assets/new.png".len());
    }

    #[test]
    fn replacing_image_source_handles_escaped_angle_target_close() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "See ![diagram](<./assets/old\\>name.png> \"Diagram\")".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState {
                anchor_byte: "See ![".len(),
                head_byte: "See ![diagram".len(),
                preferred_column: None,
                affinity: SelectionAffinity::Downstream,
            },
        });

        let snapshot = controller.snapshot();
        let (updated, selection) = replace_selected_image_source(&snapshot, "./assets/new.png")
            .expect("selected image alt should update source");

        assert_eq!(updated, "See ![diagram](./assets/new.png \"Diagram\")");
        assert_eq!(selection.cursor(), "See ![diagram](./assets/new.png".len());
    }

    #[test]
    fn replacing_image_source_handles_escaped_alt_close() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "See ![a\\]b](./assets/old.png \"Diagram\")".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState {
                anchor_byte: "See ![".len(),
                head_byte: "See ![a\\]b".len(),
                preferred_column: None,
                affinity: SelectionAffinity::Downstream,
            },
        });

        let snapshot = controller.snapshot();
        let (updated, selection) = replace_selected_image_source(&snapshot, "./assets/new.png")
            .expect("selected image alt should update source");

        assert_eq!(updated, "See ![a\\]b](./assets/new.png \"Diagram\")");
        assert_eq!(selection.cursor(), "See ![a\\]b](./assets/new.png".len());
    }

    #[test]
    fn detects_image_path_paste_over_selection() {
        let old_visible = "See diagram";
        let new_visible = "See ./assets/diagram.png";
        let selection = SelectionState {
            anchor_byte: 4,
            head_byte: 11,
            preferred_column: None,
            affinity: SelectionAffinity::Downstream,
        };

        assert_eq!(
            detect_image_paste_opportunity(old_visible, new_visible, &selection),
            Some("./assets/diagram.png".to_string())
        );
    }

    #[test]
    fn detects_image_url_paste_over_selection() {
        let old_visible = "See diagram";
        let new_visible = "See https://example.com/diagram.svg";
        let selection = SelectionState {
            anchor_byte: 4,
            head_byte: 11,
            preferred_column: None,
            affinity: SelectionAffinity::Downstream,
        };

        assert_eq!(
            detect_image_paste_opportunity(old_visible, new_visible, &selection),
            Some("https://example.com/diagram.svg".to_string())
        );
    }

    #[test]
    fn detects_markdown_autolink_image_url_paste_over_selection() {
        let old_visible = "See diagram";
        let new_visible = "See <https://example.com/diagram.svg>";
        let selection = SelectionState {
            anchor_byte: 4,
            head_byte: 11,
            preferred_column: None,
            affinity: SelectionAffinity::Downstream,
        };

        assert_eq!(
            detect_image_paste_opportunity(old_visible, new_visible, &selection),
            Some("https://example.com/diagram.svg".to_string())
        );
    }

    #[test]
    fn ignores_non_image_path_paste_over_selection() {
        let old_visible = "See diagram";
        let new_visible = "See ./assets/readme.md";
        let selection = SelectionState {
            anchor_byte: 4,
            head_byte: 11,
            preferred_column: None,
            affinity: SelectionAffinity::Downstream,
        };

        assert_eq!(
            detect_image_paste_opportunity(old_visible, new_visible, &selection),
            None
        );
    }

    #[test]
    fn ignores_url_paste_with_surrounding_whitespace() {
        let old_visible = "Read docs";
        let new_visible = "Read  https://example.com ";
        let selection = SelectionState {
            anchor_byte: 5,
            head_byte: 9,
            preferred_column: None,
            affinity: SelectionAffinity::Downstream,
        };

        assert_eq!(
            detect_link_paste_opportunity(old_visible, new_visible, &selection),
            None
        );
    }

    #[test]
    fn ignores_url_like_typing_without_selection() {
        let old_visible = "Read ";
        let new_visible = "Read https://example.com";
        let selection = SelectionState::collapsed(5);

        assert_eq!(
            detect_link_paste_opportunity(old_visible, new_visible, &selection),
            None
        );
    }

    #[test]
    fn detects_typora_highlight_wrap_over_selection() {
        let old_visible = "Mark text";
        let new_visible = "Mark =text";
        let selection = SelectionState {
            anchor_byte: 5,
            head_byte: 9,
            preferred_column: None,
            affinity: SelectionAffinity::Downstream,
        };

        assert_eq!(
            detect_wrap_selection_opportunity(old_visible, new_visible, &selection),
            Some(("==", "=="))
        );
    }

    #[test]
    fn detects_typora_superscript_wrap_over_selection() {
        let old_visible = "x 2";
        let new_visible = "x ^2";
        let selection = SelectionState {
            anchor_byte: 2,
            head_byte: 3,
            preferred_column: None,
            affinity: SelectionAffinity::Downstream,
        };

        assert_eq!(
            detect_wrap_selection_opportunity(old_visible, new_visible, &selection),
            Some(("^", "^"))
        );
    }

    #[test]
    fn detects_underscore_italic_wrap_over_selection() {
        let old_visible = "Make italic";
        let new_visible = "Make _italic";
        let selection = SelectionState {
            anchor_byte: 5,
            head_byte: 11,
            preferred_column: None,
            affinity: SelectionAffinity::Downstream,
        };

        assert_eq!(
            detect_wrap_selection_opportunity(old_visible, new_visible, &selection),
            Some(("_", "_"))
        );
    }

    #[test]
    fn detects_parenthesis_wrap_over_selection() {
        let old_visible = "Call arg";
        let new_visible = "Call (arg";
        let selection = SelectionState {
            anchor_byte: 5,
            head_byte: 8,
            preferred_column: None,
            affinity: SelectionAffinity::Downstream,
        };

        assert_eq!(
            detect_wrap_selection_opportunity(old_visible, new_visible, &selection),
            Some(("(", ")"))
        );
    }

    #[test]
    fn detects_quote_wrap_over_selection() {
        let old_visible = "Say hi";
        let new_visible = "Say \"hi";
        let selection = SelectionState {
            anchor_byte: 4,
            head_byte: 6,
            preferred_column: None,
            affinity: SelectionAffinity::Downstream,
        };

        assert_eq!(
            detect_wrap_selection_opportunity(old_visible, new_visible, &selection),
            Some(("\"", "\""))
        );
    }

    #[test]
    fn detects_angle_bracket_wrap_over_selection() {
        let old_visible = "Visit https://example.com";
        let new_visible = "Visit <https://example.com";
        let selection = SelectionState {
            anchor_byte: 6,
            head_byte: 25,
            preferred_column: None,
            affinity: SelectionAffinity::Downstream,
        };

        assert_eq!(
            detect_wrap_selection_opportunity(old_visible, new_visible, &selection),
            Some(("<", ">"))
        );
    }

    #[test]
    fn detects_closer_wrap_over_selection() {
        let old_visible = "Call docs";
        let new_visible = "Call docs)";
        let selection = SelectionState {
            anchor_byte: 5,
            head_byte: 9,
            preferred_column: None,
            affinity: SelectionAffinity::Downstream,
        };

        assert_eq!(
            detect_wrap_selection_opportunity(old_visible, new_visible, &selection),
            Some(("(", ")"))
        );
    }

    #[test]
    fn ignores_closer_inserted_before_selection_start_for_wrap() {
        let old_visible = "Call docs";
        let new_visible = "Call )docs";
        let selection = SelectionState {
            anchor_byte: 5,
            head_byte: 9,
            preferred_column: None,
            affinity: SelectionAffinity::Downstream,
        };

        assert_eq!(
            detect_wrap_selection_opportunity(old_visible, new_visible, &selection),
            None
        );
    }

    #[test]
    fn detects_markdown_inline_delimiter_auto_pairs() {
        let old_visible = "Formula ";
        let selection = SelectionState::collapsed(old_visible.len());

        assert_eq!(
            detect_auto_pair_opportunity(old_visible, "Formula $", &selection),
            Some('$')
        );
        assert_eq!(
            detect_auto_pair_opportunity(old_visible, "Formula `", &selection),
            Some('`')
        );
        assert_eq!(
            detect_auto_pair_opportunity(old_visible, "Formula *", &selection),
            Some('*')
        );
        assert_eq!(
            detect_auto_pair_opportunity(old_visible, "Formula _", &selection),
            Some('_')
        );
        assert_eq!(
            detect_auto_pair_opportunity(old_visible, "Formula ~", &selection),
            Some('~')
        );
        assert_eq!(
            detect_auto_pair_opportunity(old_visible, "Formula =", &selection),
            Some('=')
        );
        assert_eq!(
            detect_auto_pair_opportunity(old_visible, "Formula ^", &selection),
            Some('^')
        );
    }

    #[test]
    fn detects_angle_bracket_auto_pair() {
        let old_visible = "Link ";
        let selection = SelectionState::collapsed(old_visible.len());

        assert_eq!(
            detect_auto_pair_opportunity(old_visible, "Link <", &selection),
            Some('>')
        );
    }

    #[test]
    fn detects_markdown_inline_delimiter_overclose() {
        let old_visible = "$x$";
        let selection = SelectionState::collapsed(2);

        assert!(detect_overclose_opportunity(old_visible, "$x$$", &selection));
    }

    #[test]
    fn detects_markdown_emphasis_delimiter_overclose() {
        let old_visible = "*x*";
        let selection = SelectionState::collapsed(2);

        assert!(detect_overclose_opportunity(old_visible, "*x**", &selection));
    }

    #[test]
    fn detects_typora_highlight_delimiter_overclose() {
        let old_visible = "=x=";
        let selection = SelectionState::collapsed(2);

        assert!(detect_overclose_opportunity(old_visible, "=x==", &selection));
    }

    #[test]
    fn detects_angle_bracket_overclose() {
        let old_visible = "<url>";
        let selection = SelectionState::collapsed(4);

        assert!(detect_overclose_opportunity(old_visible, "<url>>", &selection));
    }

    #[test]
    fn escapes_smart_paste_link_parts() {
        assert_eq!(escape_link_label("a]b"), r"a\]b");
        assert_eq!(
            escape_link_destination("https://example.com/a)b"),
            r"https://example.com/a\)b"
        );
        assert_eq!(
            markdown_link(r"a\]b", r"https://example.com/a\)b"),
            r"[a\\\]b](https://example.com/a\\\)b)"
        );
        assert_eq!(
            markdown_link("docs", "https://example.com/guide"),
            "[docs](https://example.com/guide)"
        );
    }

    #[test]
    fn escapes_smart_paste_image_parts() {
        assert_eq!(
            markdown_image(r"a\]b", r"./assets/my pic(1).png"),
            r"![a\\\]b](./assets/my pic(1\).png)"
        );
    }

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
