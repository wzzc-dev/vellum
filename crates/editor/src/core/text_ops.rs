use std::{cmp, ops::Range};

use super::document::BlockKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SemanticEnterTransform {
    pub(crate) replacement: String,
    pub(crate) cursor_offset: usize,
}

#[derive(Debug, Clone)]
struct EditedText {
    text: String,
    cursor_offset: usize,
}

#[derive(Debug, Clone)]
struct ListLineInfo {
    current_prefix_end: usize,
    continuation_prefix: String,
    is_empty: bool,
}

#[derive(Debug, Clone)]
struct QuoteLineInfo {
    current_prefix_end: usize,
    continuation_prefix: String,
    is_empty: bool,
}

pub(crate) fn count_document_words(text: &str) -> usize {
    let mut count = 0usize;
    let mut in_word = false;

    for ch in text.chars() {
        if is_cjk_character(ch) {
            if in_word {
                count += 1;
                in_word = false;
            }
            count += 1;
        } else if ch.is_alphanumeric() {
            in_word = true;
        } else if in_word {
            count += 1;
            in_word = false;
        }
    }

    if in_word {
        count += 1;
    }

    count
}

pub(crate) fn adjust_block_markup(text: &str, deepen: bool) -> Option<String> {
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

    if trimmed.starts_with('>') {
        let updated = text
            .lines()
            .map(|line| {
                let line_trimmed = line.trim_start();
                let line_indent = &line[..line.len().saturating_sub(line_trimmed.len())];

                if deepen {
                    format!("{line_indent}> {line_trimmed}")
                } else {
                    let stripped = line_trimmed
                        .strip_prefix("> ")
                        .or_else(|| line_trimmed.strip_prefix('>'))
                        .unwrap_or(line_trimmed);
                    format!("{line_indent}{stripped}")
                }
            })
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

pub(crate) fn supports_semantic_enter(kind: &BlockKind) -> bool {
    matches!(
        kind,
        BlockKind::Raw
            | BlockKind::Paragraph
            | BlockKind::Heading { .. }
            | BlockKind::List
            | BlockKind::Blockquote
    )
}

pub(crate) fn semantic_enter_transform(
    kind: &BlockKind,
    text: &str,
    selection: Option<Range<usize>>,
    cursor_offset: usize,
) -> Option<SemanticEnterTransform> {
    if !supports_semantic_enter(kind) {
        return None;
    }

    let edited = apply_selection(text, selection, cursor_offset);
    match kind {
        BlockKind::Raw | BlockKind::Paragraph | BlockKind::Heading { .. } => {
            Some(split_block_transform(&edited.text, edited.cursor_offset))
        }
        BlockKind::List => list_enter_transform(&edited.text, edited.cursor_offset),
        BlockKind::Blockquote => blockquote_enter_transform(&edited.text, edited.cursor_offset),
        _ => None,
    }
}

pub(crate) fn byte_offset_for_line_column(
    text: &str,
    target_line: usize,
    target_column: usize,
) -> usize {
    let mut offset = 0usize;

    for (line_ix, segment) in text.split('\n').enumerate() {
        if line_ix == target_line {
            return offset + byte_offset_for_char_column(segment, target_column);
        }

        offset += segment.len();
        if offset < text.len() {
            offset += 1;
        }
    }

    text.len()
}

pub(crate) fn line_column_for_byte_offset(text: &str, target_offset: usize) -> (usize, usize) {
    let offset = clamp_to_char_boundary(text, target_offset);
    let prefix = &text[..offset];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count();
    let column = prefix
        .rsplit_once('\n')
        .map(|(_, tail)| tail.chars().count())
        .unwrap_or_else(|| prefix.chars().count());

    (line, column)
}

pub(crate) fn clamp_to_char_boundary(text: &str, offset: usize) -> usize {
    let mut offset = offset.min(text.len());
    while offset > 0 && !text.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

pub(crate) fn utf16_range_to_byte_range(text: &str, range: &Range<usize>) -> Range<usize> {
    utf16_offset_to_byte_offset(text, range.start)..utf16_offset_to_byte_offset(text, range.end)
}

pub(crate) fn utf16_offset_to_byte_offset(text: &str, target: usize) -> usize {
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

pub(crate) fn compute_document_diff(old: &str, new: &str) -> Option<(Range<usize>, String)> {
    if old == new {
        return None;
    }

    let mut prefix = common_prefix_len(old.as_bytes(), new.as_bytes());
    while prefix > 0 && (!old.is_char_boundary(prefix) || !new.is_char_boundary(prefix)) {
        prefix -= 1;
    }

    let old_remaining = &old.as_bytes()[prefix..];
    let new_remaining = &new.as_bytes()[prefix..];
    let mut suffix = common_suffix_len(old_remaining, new_remaining);
    while suffix > 0 {
        let old_start = old.len().saturating_sub(suffix);
        let new_start = new.len().saturating_sub(suffix);
        if old.is_char_boundary(old_start) && new.is_char_boundary(new_start) {
            break;
        }
        suffix -= 1;
    }

    let old_end = old.len().saturating_sub(suffix);
    let new_end = new.len().saturating_sub(suffix);
    Some((prefix..old_end, new[prefix..new_end].to_string()))
}

fn byte_offset_for_char_column(text: &str, target_column: usize) -> usize {
    match text.char_indices().nth(target_column) {
        Some((offset, _)) => offset,
        None => text.len(),
    }
}

fn split_block_transform(text: &str, cursor_offset: usize) -> SemanticEnterTransform {
    let cursor_offset = clamp_to_char_boundary(text, cursor_offset);
    let before = &text[..cursor_offset];
    let after = &text[cursor_offset..];
    SemanticEnterTransform {
        replacement: format!("{before}\n\n{after}"),
        cursor_offset: before.len() + 2,
    }
}

fn list_enter_transform(text: &str, cursor_offset: usize) -> Option<SemanticEnterTransform> {
    let cursor_offset = clamp_to_char_boundary(text, cursor_offset);
    let (line_start, line_end) = line_bounds(text, cursor_offset);
    let line = &text[line_start..line_end];
    let info = parse_list_line(line)?;

    if info.is_empty {
        return Some(exit_structured_line(text, line_start, line_end));
    }

    let split_offset = cursor_offset.max(line_start + info.current_prefix_end);
    let local_split = split_offset - line_start;
    let current_line = &line[..local_split];
    let moved_suffix = &line[local_split..];
    let before = &text[..line_start];
    let after = &text[line_end..];
    let replacement = format!(
        "{before}{current_line}\n{}{}{after}",
        info.continuation_prefix, moved_suffix
    );
    let cursor_offset = before.len() + current_line.len() + 1 + info.continuation_prefix.len();

    Some(SemanticEnterTransform {
        replacement,
        cursor_offset,
    })
}

fn blockquote_enter_transform(text: &str, cursor_offset: usize) -> Option<SemanticEnterTransform> {
    let cursor_offset = clamp_to_char_boundary(text, cursor_offset);
    let (line_start, line_end) = line_bounds(text, cursor_offset);
    let line = &text[line_start..line_end];
    let info = parse_blockquote_line(line)?;

    if info.is_empty {
        return Some(exit_structured_line(text, line_start, line_end));
    }

    let split_offset = cursor_offset.max(line_start + info.current_prefix_end);
    let local_split = split_offset - line_start;
    let current_line = &line[..local_split];
    let moved_suffix = &line[local_split..];
    let before = &text[..line_start];
    let after = &text[line_end..];
    let replacement = format!(
        "{before}{current_line}\n{}{}{after}",
        info.continuation_prefix, moved_suffix
    );
    let cursor_offset = before.len() + current_line.len() + 1 + info.continuation_prefix.len();

    Some(SemanticEnterTransform {
        replacement,
        cursor_offset,
    })
}

fn exit_structured_line(text: &str, line_start: usize, line_end: usize) -> SemanticEnterTransform {
    let before = trim_trailing_newlines(&text[..line_start]);
    let after = trim_leading_newlines(&text[line_end..]);

    let (replacement, cursor_offset) = match (before.is_empty(), after.is_empty()) {
        (true, true) => (String::new(), 0),
        (true, false) => (after.to_string(), 0),
        (false, true) => (format!("{before}\n\n"), before.len() + 2),
        (false, false) => (format!("{before}\n\n{after}"), before.len() + 2),
    };

    SemanticEnterTransform {
        replacement,
        cursor_offset,
    }
}

fn apply_selection(
    text: &str,
    selection: Option<Range<usize>>,
    cursor_offset: usize,
) -> EditedText {
    let cursor_offset = clamp_to_char_boundary(text, cursor_offset);
    let Some(selection) = selection.filter(|selection| !selection.is_empty()) else {
        return EditedText {
            text: text.to_string(),
            cursor_offset,
        };
    };

    let start = clamp_to_char_boundary(text, selection.start);
    let end = clamp_to_char_boundary(text, selection.end.max(start));
    let replacement = format!("{}{}", &text[..start], &text[end..]);

    EditedText {
        text: replacement,
        cursor_offset: start,
    }
}

fn line_bounds(text: &str, cursor_offset: usize) -> (usize, usize) {
    let line_start = text[..cursor_offset]
        .rfind('\n')
        .map(|ix| ix + 1)
        .unwrap_or(0);
    let line_end = text[cursor_offset..]
        .find('\n')
        .map(|ix| cursor_offset + ix)
        .unwrap_or(text.len());
    (line_start, line_end)
}

fn parse_list_line(line: &str) -> Option<ListLineInfo> {
    let indent_end = line
        .bytes()
        .take_while(|byte| matches!(byte, b' ' | b'\t'))
        .count();
    let indent = &line[..indent_end];
    let trimmed = &line[indent_end..];

    for marker in [
        "- [ ] ", "* [ ] ", "+ [ ] ", "- [x] ", "* [x] ", "+ [x] ", "- [X] ", "* [X] ", "+ [X] ",
    ] {
        if trimmed.starts_with(marker) {
            let bullet = &marker[..1];
            let current_prefix_end = indent_end + marker.len();
            let continuation_prefix = format!("{indent}{bullet} [ ] ");
            let is_empty = line[current_prefix_end..].trim().is_empty();
            return Some(ListLineInfo {
                current_prefix_end,
                continuation_prefix,
                is_empty,
            });
        }
    }

    for marker in ["- ", "* ", "+ "] {
        if trimmed.starts_with(marker) {
            let current_prefix_end = indent_end + marker.len();
            let continuation_prefix = format!("{indent}{marker}");
            let is_empty = line[current_prefix_end..].trim().is_empty();
            return Some(ListLineInfo {
                current_prefix_end,
                continuation_prefix,
                is_empty,
            });
        }
    }

    if let Some((number, _)) = trimmed.split_once(". ")
        && !number.is_empty()
        && number.chars().all(|ch| ch.is_ascii_digit())
    {
        let current_prefix_end = indent_end + number.len() + 2;
        let next_number = number.parse::<usize>().unwrap_or(1).saturating_add(1);
        let continuation_prefix = format!("{indent}{next_number}. ");
        let is_empty = line[current_prefix_end..].trim().is_empty();
        return Some(ListLineInfo {
            current_prefix_end,
            continuation_prefix,
            is_empty,
        });
    }

    None
}

fn parse_blockquote_line(line: &str) -> Option<QuoteLineInfo> {
    let bytes = line.as_bytes();
    let mut ix = 0usize;
    while ix < bytes.len() && matches!(bytes[ix], b' ' | b'\t') {
        ix += 1;
    }

    let mut saw_marker = false;
    while ix < bytes.len() && bytes[ix] == b'>' {
        saw_marker = true;
        ix += 1;
        while ix < bytes.len() && matches!(bytes[ix], b' ' | b'\t') {
            ix += 1;
        }
    }

    if !saw_marker {
        return None;
    }

    let current_prefix_end = ix;
    let mut continuation_prefix = line[..current_prefix_end].to_string();
    if !continuation_prefix.ends_with(' ') {
        continuation_prefix.push(' ');
    }
    let is_empty = line[current_prefix_end..].trim().is_empty();

    Some(QuoteLineInfo {
        current_prefix_end,
        continuation_prefix,
        is_empty,
    })
}

fn trim_leading_newlines(text: &str) -> &str {
    text.trim_start_matches(['\r', '\n'])
}

fn trim_trailing_newlines(text: &str) -> &str {
    text.trim_end_matches(['\r', '\n'])
}

fn is_cjk_character(ch: char) -> bool {
    matches!(
        ch as u32,
        0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0x3040..=0x30FF
            | 0x31F0..=0x31FF
            | 0xAC00..=0xD7AF
    )
}

fn common_prefix_len(left: &[u8], right: &[u8]) -> usize {
    left.iter()
        .zip(right.iter())
        .take_while(|(left, right)| left == right)
        .count()
}

fn common_suffix_len(left: &[u8], right: &[u8]) -> usize {
    left.iter()
        .rev()
        .zip(right.iter().rev())
        .take_while(|(left, right)| left == right)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_words_across_cjk_and_ascii() {
        assert_eq!(count_document_words("hello world"), 2);
        assert_eq!(count_document_words("你好 world"), 3);
    }

    #[test]
    fn adjusts_heading_markup() {
        assert_eq!(
            adjust_block_markup("# Title", true),
            Some("## Title".to_string())
        );
        assert_eq!(
            adjust_block_markup("## Title", false),
            Some("# Title".to_string())
        );
    }

    #[test]
    fn deepens_plain_text_into_heading() {
        assert_eq!(
            adjust_block_markup("Title", true),
            Some("# Title".to_string())
        );
    }

    #[test]
    fn maps_line_and_column_back_to_utf8_offset() {
        assert_eq!(byte_offset_for_line_column("abc\ndef", 0, 0), 0);
        assert_eq!(byte_offset_for_line_column("abc\ndef", 0, 2), 2);
        assert_eq!(byte_offset_for_line_column("abc\ndef", 1, 1), 5);
        assert_eq!(byte_offset_for_line_column("a\nworld", 1, 3), 5);
    }

    #[test]
    fn maps_utf8_offset_back_to_line_and_column() {
        assert_eq!(line_column_for_byte_offset("abc\ndef", 0), (0, 0));
        assert_eq!(line_column_for_byte_offset("abc\ndef", 2), (0, 2));
        assert_eq!(line_column_for_byte_offset("abc\ndef", 4), (1, 0));
        assert_eq!(line_column_for_byte_offset("abc\ndef", 7), (1, 3));
    }

    #[test]
    fn compute_document_diff_preserves_utf8_boundaries() {
        let diff = compute_document_diff("A🙂中B", "A🙂文B").unwrap();

        assert_eq!(diff.0, "A🙂".len().."A🙂中".len());
        assert_eq!(diff.1, "文");
    }

    #[test]
    fn maps_utf16_offset_back_to_utf8_offset() {
        let text = "A\u{1F642}\u{65B0}";

        assert_eq!(utf16_offset_to_byte_offset(text, 0), 0);
        assert_eq!(utf16_offset_to_byte_offset(text, 1), 1);
        assert_eq!(utf16_offset_to_byte_offset(text, 3), "A\u{1F642}".len());
        assert_eq!(utf16_offset_to_byte_offset(text, 4), text.len());
    }

    #[test]
    fn maps_utf16_range_back_to_utf8_range() {
        let text = "A\u{1F642}\u{65B0}";

        assert_eq!(utf16_range_to_byte_range(text, &(1..4)), 1..text.len());
    }

    #[test]
    fn splits_paragraph_on_semantic_enter() {
        let transform =
            semantic_enter_transform(&BlockKind::Paragraph, "alpha beta", None, 5).unwrap();

        assert_eq!(transform.replacement, "alpha\n\n beta");
        assert_eq!(transform.cursor_offset, 7);
    }

    #[test]
    fn splits_heading_into_following_paragraph() {
        let transform =
            semantic_enter_transform(&BlockKind::Heading { depth: 1 }, "# Title", None, 3).unwrap();

        assert_eq!(transform.replacement, "# T\n\nitle");
        assert_eq!(transform.cursor_offset, 5);
    }

    #[test]
    fn continues_list_item_and_moves_remainder() {
        let transform = semantic_enter_transform(&BlockKind::List, "- item", None, 3).unwrap();

        assert_eq!(transform.replacement, "- i\n- tem");
        assert_eq!(transform.cursor_offset, 6);
    }

    #[test]
    fn exits_empty_task_list_item() {
        let transform =
            semantic_enter_transform(&BlockKind::List, "- one\n- [ ] ", None, 11).unwrap();

        assert_eq!(transform.replacement, "- one\n\n");
        assert_eq!(transform.cursor_offset, 7);
    }

    #[test]
    fn continues_blockquote_line() {
        let transform =
            semantic_enter_transform(&BlockKind::Blockquote, "> quoted", None, 5).unwrap();

        assert_eq!(transform.replacement, "> quo\n> ted");
        assert_eq!(transform.cursor_offset, 8);
    }

    #[test]
    fn exits_empty_blockquote_line() {
        let transform =
            semantic_enter_transform(&BlockKind::Blockquote, "> keep\n> ", None, 9).unwrap();

        assert_eq!(transform.replacement, "> keep\n\n");
        assert_eq!(transform.cursor_offset, 8);
    }

    #[test]
    fn semantic_enter_replaces_selection_before_splitting() {
        let transform =
            semantic_enter_transform(&BlockKind::Paragraph, "alpha beta", Some(2..7), 7).unwrap();

        assert_eq!(transform.replacement, "al\n\neta");
        assert_eq!(transform.cursor_offset, 4);
    }

    #[test]
    fn semantic_enter_only_targets_supported_body_blocks() {
        assert!(supports_semantic_enter(&BlockKind::Paragraph));
        assert!(supports_semantic_enter(&BlockKind::List));
        assert!(!supports_semantic_enter(&BlockKind::Table));
        assert!(!supports_semantic_enter(&BlockKind::CodeFence {
            language: Some("rust".to_string()),
        }));
    }

    #[test]
    fn adjusts_blockquote_markup() {
        assert_eq!(
            adjust_block_markup("> Quote", false),
            Some("Quote".to_string())
        );
        assert_eq!(
            adjust_block_markup("> Quote", true),
            Some("> > Quote".to_string())
        );
    }
}
