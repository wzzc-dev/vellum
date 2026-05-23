use std::{cmp, ops::Range};

use super::{document::BlockKind, table::pipe_row_cell_count};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SemanticEnterTransform {
    pub(crate) replacement: String,
    pub(crate) cursor_offset: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SelectionTransform {
    pub(crate) replacement: String,
    pub(crate) selection: Range<usize>,
}

#[derive(Debug, Clone)]
struct EditedText {
    text: String,
    cursor_offset: usize,
}

#[derive(Debug, Clone)]
struct ListLineInfo {
    indent_len: usize,
    current_prefix_end: usize,
    continuation_prefix: String,
    ordered_marker: Option<OrderedListMarker>,
    is_empty: bool,
    is_task: bool,
    marker_needs_space: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OrderedListMarker {
    number: usize,
    delimiter: char,
    prefix_len: usize,
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

    if matches!(trimmed.as_bytes(), [b'-' | b'*' | b'+', b' ', ..])
        || strip_task_marker(trimmed).is_some()
        || ordered_list_marker(trimmed).is_some()
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

pub(crate) fn adjust_quoted_list_markup_at_cursor(
    text: &str,
    cursor_offset: usize,
    deepen: bool,
) -> Option<SemanticEnterTransform> {
    let cursor_offset = clamp_to_char_boundary(text, cursor_offset);
    let (line_start, line_end) = line_bounds(text, cursor_offset);
    let line = &text[line_start..line_end];
    let quote_prefix_end = quoted_content_prefix_end(line)?;
    let inner = &line[quote_prefix_end..];
    let list_info = parse_list_line(inner)?;

    if !deepen && list_info.indent_len < 2 {
        let updated_line = format!(
            "{}{}",
            &line[..quote_prefix_end],
            &inner[list_info.current_prefix_end..]
        );
        let replacement = format!("{}{}{}", &text[..line_start], updated_line, &text[line_end..]);
        let marker_len = list_info.current_prefix_end;
        let marker_start = line_start + quote_prefix_end;
        let cursor_offset = if cursor_offset > marker_start {
            cursor_offset.saturating_sub(marker_len)
        } else {
            cursor_offset
        };
        return Some(SemanticEnterTransform {
            replacement,
            cursor_offset,
        });
    }

    let inner_indent = &inner[..list_info.indent_len];
    let updated_inner_indent = if deepen {
        format!("{inner_indent}  ")
    } else {
        inner_indent[..inner_indent.len() - 2].to_string()
    };
    let updated_line = format!(
        "{}{}{}",
        &line[..quote_prefix_end],
        updated_inner_indent,
        &inner[list_info.indent_len..]
    );
    let replacement = format!("{}{}{}", &text[..line_start], updated_line, &text[line_end..]);

    let old_content_start = line_start + quote_prefix_end + list_info.indent_len;
    let delta = updated_inner_indent.len() as isize - inner_indent.len() as isize;
    let cursor_offset = if cursor_offset >= old_content_start {
        cursor_offset.saturating_add_signed(delta)
    } else {
        cursor_offset
    };

    Some(SemanticEnterTransform {
        replacement,
        cursor_offset,
    })
}

pub(crate) fn adjust_list_markup_at_cursor(
    text: &str,
    cursor_offset: usize,
    deepen: bool,
) -> Option<SemanticEnterTransform> {
    let cursor_offset = clamp_to_char_boundary(text, cursor_offset);
    let (line_start, line_end) = line_bounds(text, cursor_offset);
    let line = &text[line_start..line_end];
    let list_info = parse_list_line(line)?;

    let indent = &line[..list_info.indent_len];
    if !deepen && list_info.indent_len < 2 {
        let updated_line = line[list_info.current_prefix_end..].to_string();
        let replacement = format!("{}{}{}", &text[..line_start], updated_line, &text[line_end..]);
        let cursor_offset = cursor_offset
            .saturating_sub(list_info.current_prefix_end)
            .max(line_start);
        return Some(SemanticEnterTransform {
            replacement,
            cursor_offset,
        });
    }

    let updated_indent = if deepen {
        format!("{indent}  ")
    } else {
        indent[..indent.len() - 2].to_string()
    };
    let updated_line = format!("{}{}", updated_indent, &line[list_info.indent_len..]);
    let replacement = format!("{}{}{}", &text[..line_start], updated_line, &text[line_end..]);

    let old_content_start = line_start + list_info.indent_len;
    let delta = updated_indent.len() as isize - indent.len() as isize;
    let cursor_offset = if cursor_offset >= old_content_start {
        cursor_offset.saturating_add_signed(delta)
    } else {
        cursor_offset
    };

    Some(SemanticEnterTransform {
        replacement,
        cursor_offset,
    })
}

pub(crate) fn adjust_selected_list_markup(
    text: &str,
    selection: Range<usize>,
    deepen: bool,
) -> Option<SelectionTransform> {
    adjust_selected_lines(text, selection, deepen, adjusted_list_line)
}

pub(crate) fn adjust_selected_quoted_list_markup(
    text: &str,
    selection: Range<usize>,
    deepen: bool,
) -> Option<SelectionTransform> {
    adjust_selected_lines(text, selection, deepen, adjusted_quoted_list_line)
}

fn adjust_selected_lines(
    text: &str,
    selection: Range<usize>,
    deepen: bool,
    adjust_line: fn(&str, bool) -> Option<String>,
) -> Option<SelectionTransform> {
    let start = clamp_to_char_boundary(text, selection.start.min(selection.end));
    let end = clamp_to_char_boundary(text, selection.end.max(selection.start));
    if start == end {
        return None;
    }

    let (start_line, _) = line_bounds(text, start);
    let mut end_line_offset = end;
    if end_line_offset > start_line && text.as_bytes().get(end_line_offset - 1) == Some(&b'\n') {
        end_line_offset -= 1;
    }
    let (_, end_line) = line_bounds(text, end_line_offset);

    let mut replacement = String::with_capacity(text.len() + 2);
    replacement.push_str(&text[..start_line]);

    let mut cursor = start_line;
    let mut changed = false;
    let mut start_delta = 0isize;
    let mut end_delta = 0isize;
    while cursor <= end_line {
        let line_end = text[cursor..]
            .find('\n')
            .map(|ix| cursor + ix)
            .unwrap_or(text.len());
        let line = &text[cursor..line_end];
        let before_len = replacement.len();
        if let Some(updated_line) = adjust_line(line, deepen) {
            changed = true;
            let delta = updated_line.len() as isize - line.len() as isize;
            if cursor < start {
                start_delta += delta;
            }
            if cursor < end {
                end_delta += delta;
            }
            replacement.push_str(&updated_line);
        } else {
            replacement.push_str(line);
        }
        if line_end < text.len() && line_end < end_line {
            replacement.push('\n');
        }
        cursor = line_end.saturating_add(1);
        if before_len == replacement.len() && line_end >= text.len() {
            break;
        }
        if line_end >= end_line {
            break;
        }
    }

    if !changed {
        return None;
    }

    replacement.push_str(&text[end_line..]);
    let selection_start = start.saturating_add_signed(start_delta);
    let selection_end = end.saturating_add_signed(end_delta).max(selection_start);

    Some(SelectionTransform {
        replacement,
        selection: selection_start..selection_end,
    })
}

fn adjusted_quoted_list_line(line: &str, deepen: bool) -> Option<String> {
    let quote_prefix_end = quoted_content_prefix_end(line)?;
    let updated_inner = adjusted_list_line(&line[quote_prefix_end..], deepen)?;
    Some(format!("{}{}", &line[..quote_prefix_end], updated_inner))
}

fn adjusted_list_line(line: &str, deepen: bool) -> Option<String> {
    let list_info = parse_list_line(line)?;
    let indent = &line[..list_info.indent_len];
    if !deepen && list_info.indent_len < 2 {
        return Some(line[list_info.current_prefix_end..].to_string());
    }

    let updated_indent = if deepen {
        format!("{indent}  ")
    } else {
        indent[..indent.len() - 2].to_string()
    };
    Some(format!(
        "{}{}",
        updated_indent,
        &line[list_info.indent_len..]
    ))
}

/// Set the block's heading marker to the given depth (1–6), or strip it to plain
/// paragraph text if `depth == 0`. Works on both heading and non-heading blocks.
pub(crate) fn set_heading_markup(text: &str, depth: u8) -> String {
    let mut lines = text.lines();
    let Some(first) = lines.next() else {
        if depth == 0 {
            return String::new();
        }
        return format!("{} ", "#".repeat(depth as usize));
    };
    let rest = if text.contains('\n') {
        text[first.len()..].to_string()
    } else {
        String::new()
    };

    let trimmed = first.trim_start();
    let indent = &first[..first.len().saturating_sub(trimmed.len())];

    // Strip any existing heading marker to get bare content.
    let content = if let Some(space_ix) = trimmed.find(' ') {
        let marker = &trimmed[..space_ix];
        if marker.chars().all(|ch| ch == '#') && !marker.is_empty() {
            &trimmed[space_ix + 1..]
        } else {
            trimmed
        }
    } else {
        trimmed
    };

    let head = if depth == 0 {
        format!("{indent}{content}")
    } else {
        format!("{indent}{} {content}", "#".repeat(depth as usize))
    };
    format!("{head}{rest}")
}

/// Toggle the blockquote prefix (`> `) on every line of `text`.
/// If `enabled` is true, prepend `> ` to every line; if false, strip it.
pub(crate) fn set_blockquote_markup(text: &str, enabled: bool) -> String {
    if text.is_empty() {
        if enabled {
            return "> ".to_string();
        }
        return String::new();
    }
    text.lines()
        .map(|line| {
            if enabled {
                format!("> {line}")
            } else {
                // strip one level of `> ` or `>`
                line.strip_prefix("> ")
                    .or_else(|| line.strip_prefix('>'))
                    .unwrap_or(line)
                    .to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Convert `text` to a bullet list (`- content`) or an ordered list (`1. content`),
/// or strip existing list markup back to plain paragraph text.
/// If `text` is already the requested kind, strips it to paragraph instead (toggle).
///
/// `ordered`: true → `1.` style, false → `-` style.
pub(crate) fn set_list_markup(text: &str, ordered: bool) -> String {
    let list_markers_unordered = ["- ", "* ", "+ "];
    let task_markers = [
        "- [ ] ", "- [x] ", "- [X] ", "* [ ] ", "* [x] ", "* [X] ", "+ [ ] ",
        "+ [x] ", "+ [X] ",
    ];

    let mut lines = text.lines();
    let Some(first) = lines.next() else {
        if ordered {
            return "1. ".to_string();
        }
        return "- ".to_string();
    };

    let trimmed = first.trim_start();
    let _indent = &first[..first.len().saturating_sub(trimmed.len())];

    // Detect current kind.
    let is_unordered = task_markers
        .iter()
        .chain(list_markers_unordered.iter())
        .any(|m| trimmed.starts_with(m));
    let is_ordered = ordered_list_marker(trimmed).is_some();

    // Toggle off if already the requested kind.
    if (ordered && is_ordered) || (!ordered && is_unordered) {
        // Strip list prefix from all lines.
        return text
            .lines()
            .map(|line| {
                let t = line.trim_start();
                let ind = &line[..line.len().saturating_sub(t.len())];
                // strip ordered
                if let Some(marker) = ordered_list_marker(t) {
                    return format!("{ind}{}", &t[marker.prefix_len..]);
                }
                // strip unordered / task
                for marker in task_markers.iter().chain(list_markers_unordered.iter()) {
                    if let Some(rest) = t.strip_prefix(marker) {
                        return format!("{ind}{rest}");
                    }
                }
                line.to_string()
            })
            .collect::<Vec<_>>()
            .join("\n");
    }

    // Convert: strip existing marker first, then add new one.
    let mut counter = 1usize;
    let all_lines: Vec<&str> = text.lines().collect();
    all_lines
        .iter()
        .map(|line| {
            let t = line.trim_start();
            let ind = &line[..line.len().saturating_sub(t.len())];
            // bare content after stripping any existing marker
            let content = 'strip: {
                if let Some(marker) = ordered_list_marker(t) {
                    break 'strip &t[marker.prefix_len..];
                }
                for marker in task_markers.iter().chain(list_markers_unordered.iter()) {
                    if let Some(rest) = t.strip_prefix(marker) {
                        break 'strip rest;
                    }
                }
                t
            };
            let new_line = if ordered {
                let result = format!("{ind}{}. {content}", counter);
                counter += 1;
                result
            } else {
                format!("{ind}- {content}")
            };
            new_line
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn set_task_list_markup(text: &str) -> String {
    if text.is_empty() {
        return "- [ ] ".to_string();
    }

    text.lines()
        .map(set_task_list_line_markup)
        .collect::<Vec<_>>()
        .join("\n")
}

fn set_task_list_line_markup(line: &str) -> String {
    let trimmed = line.trim_start();
    let indent = &line[..line.len().saturating_sub(trimmed.len())];
    let unordered_markers = ["- ", "* ", "+ "];

    let mut checked = false;
    let content = 'strip: {
        if let Some(rest) = strip_task_marker(trimmed) {
            checked = task_marker_is_checked(trimmed).unwrap_or(false);
            break 'strip rest;
        }
        if let Some(marker) = ordered_list_marker(trimmed) {
            break 'strip &trimmed[marker.prefix_len..];
        }
        for marker in unordered_markers {
            if let Some(rest) = trimmed.strip_prefix(marker) {
                break 'strip rest;
            }
        }
        trimmed
    };

    let marker = if checked { "- [x]" } else { "- [ ]" };
    format!("{indent}{marker} {content}")
}

fn task_marker_is_checked(trimmed: &str) -> Option<bool> {
    let bytes = trimmed.as_bytes();
    if bytes.len() < 5
        || !matches!(bytes[0], b'-' | b'*' | b'+')
        || bytes[1] != b' '
        || bytes[2] != b'['
        || bytes[4] != b']'
    {
        return None;
    }

    match bytes[3] {
        b' ' => Some(false),
        b'x' | b'X' => Some(true),
        _ => None,
    }
}

fn strip_task_marker(trimmed: &str) -> Option<&str> {
    let bytes = trimmed.as_bytes();
    if bytes.len() < 5 {
        return None;
    }
    if !matches!(bytes[0], b'-' | b'*' | b'+')
        || bytes[1] != b' '
        || bytes[2] != b'['
        || !matches!(bytes[3], b' ' | b'x' | b'X')
        || bytes[4] != b']'
    {
        return None;
    }

    if bytes.len() == 5 {
        return Some("");
    }
    if bytes.get(5).is_some_and(|byte| byte.is_ascii_whitespace()) {
        return Some(trimmed[6..].trim_start_matches([' ', '\t']));
    }

    None
}

fn looks_like_incomplete_task_marker(trimmed: &str) -> bool {
    let bytes = trimmed.as_bytes();
    bytes.len() >= 5
        && matches!(bytes[0], b'-' | b'*' | b'+')
        && bytes[1] == b' '
        && bytes[2] == b'['
        && matches!(bytes[3], b' ' | b'x' | b'X')
        && bytes[4] == b']'
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AutoFormatAction {
    Heading { depth: u8 },
    Blockquote,
    BulletList,
    OrderedList,
    TaskList,
    HorizontalRule,
    CodeFence { marker: String, info: String },
    MathBlock,
}

pub(crate) fn detect_auto_format(kind: &BlockKind, text: &str) -> Option<AutoFormatAction> {
    if !matches!(kind, BlockKind::Raw | BlockKind::Paragraph) {
        return None;
    }

    let trimmed = text.trim();

    if trimmed.is_empty() {
        return None;
    }

    if trimmed == "$$" {
        return Some(AutoFormatAction::MathBlock);
    }

    if let Some(action) = detect_code_fence_auto_format(trimmed) {
        return Some(action);
    }

    if trimmed.len() >= 3 {
        let first = trimmed.chars().next()?;
        if (first == '-' || first == '*' || first == '_') && trimmed.chars().all(|c| c == first) {
            return Some(AutoFormatAction::HorizontalRule);
        }
    }

    {
        let line_start = text.trim_start();
        let hashes: String = line_start.chars().take_while(|c| *c == '#').collect();
        let depth = hashes.len() as u8;
        if depth >= 1 && depth <= 6 {
            let after_hashes = line_start.get(hashes.len()..)?;
            if after_hashes.starts_with(' ') {
                return Some(AutoFormatAction::Heading { depth });
            }
        }
    }

    let line_start = text.trim_start();
    if line_start.starts_with('>') {
        return Some(AutoFormatAction::Blockquote);
    }

    if strip_task_marker(line_start).is_some() {
        return Some(AutoFormatAction::TaskList);
    }
    if looks_like_incomplete_task_marker(line_start) {
        return None;
    }

    if line_start.starts_with("- ") || line_start.starts_with("* ") || line_start.starts_with("+ ")
    {
        return Some(AutoFormatAction::BulletList);
    }

    if ordered_list_marker(line_start).is_some() {
        return Some(AutoFormatAction::OrderedList);
    }

    None
}

fn detect_code_fence_auto_format(trimmed: &str) -> Option<AutoFormatAction> {
    let marker_char = trimmed.chars().next()?;
    if marker_char != '`' && marker_char != '~' {
        return None;
    }

    let marker_len = trimmed
        .chars()
        .take_while(|ch| *ch == marker_char)
        .count();
    if marker_len < 3 {
        return None;
    }

    let marker = marker_char.to_string().repeat(marker_len);
    let info = trimmed[marker_len..].trim().to_string();
    if info.chars().any(|ch| ch == marker_char) {
        return None;
    }

    Some(AutoFormatAction::CodeFence { marker, info })
}

pub(crate) fn supports_semantic_enter(kind: &BlockKind) -> bool {
    matches!(
        kind,
        BlockKind::Raw
            | BlockKind::Paragraph
            | BlockKind::Heading { .. }
            | BlockKind::List
            | BlockKind::Blockquote
            | BlockKind::Callout { .. }
            | BlockKind::CodeFence { .. }
            | BlockKind::MathBlock
            | BlockKind::ThematicBreak
            | BlockKind::Toc
            | BlockKind::FootnoteDefinition
            | BlockKind::LinkReferenceDefinition
            | BlockKind::YamlFrontMatter
            | BlockKind::Html
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
        BlockKind::Raw | BlockKind::Paragraph => {
            Some(split_block_transform(&edited.text, edited.cursor_offset))
        }
        BlockKind::Heading { .. } => {
            Some(heading_enter_transform(&edited.text, edited.cursor_offset))
        }
        BlockKind::List => list_enter_transform(&edited.text, edited.cursor_offset),
        BlockKind::Blockquote | BlockKind::Callout { .. } => {
            blockquote_enter_transform(&edited.text, edited.cursor_offset)
        }
        BlockKind::CodeFence { .. } => {
            code_fence_enter_transform(&edited.text, edited.cursor_offset)
        }
        BlockKind::MathBlock => math_block_enter_transform(&edited.text, edited.cursor_offset),
        BlockKind::ThematicBreak => Some(thematic_break_enter_transform(
            &edited.text,
            edited.cursor_offset,
        )),
        BlockKind::Toc => Some(toc_enter_transform(&edited.text, edited.cursor_offset)),
        BlockKind::FootnoteDefinition
        | BlockKind::LinkReferenceDefinition
        | BlockKind::YamlFrontMatter
        | BlockKind::Html => Some(structural_block_enter_transform(
            &edited.text,
            edited.cursor_offset,
        )),
        _ => None,
    }
}

pub(crate) fn pipe_table_enter_transform(
    kind: &BlockKind,
    text: &str,
    selection: Option<Range<usize>>,
    cursor_offset: usize,
) -> Option<SemanticEnterTransform> {
    if !matches!(kind, BlockKind::Raw | BlockKind::Paragraph) {
        return None;
    }
    if selection
        .as_ref()
        .is_some_and(|selection| !selection.is_empty())
    {
        return None;
    }

    let cursor_offset = clamp_to_char_boundary(text, cursor_offset);
    if cursor_offset != text.len() {
        return None;
    }
    if text.contains(['\n', '\r']) {
        return None;
    }

    let header = normalized_pipe_table_header(text)?;
    let cells = pipe_row_cell_count(&header);
    if cells < 2 {
        return None;
    }

    let delimiter = pipe_table_delimiter_row(cells);
    let empty_row = pipe_table_empty_row(cells);
    let replacement = format!("{header}\n{delimiter}\n{empty_row}");
    let cursor_offset = header.len() + 1 + delimiter.len() + 1 + 1;

    Some(SemanticEnterTransform {
        replacement,
        cursor_offset,
    })
}

fn normalized_pipe_table_header(text: &str) -> Option<String> {
    if text.starts_with('|') && text.ends_with('|') {
        return Some(text.to_string());
    }

    let trimmed = text.trim();
    if trimmed.starts_with('|') || trimmed.ends_with('|') || unescaped_pipe_count(trimmed) == 0 {
        return None;
    }

    Some(format!("| {trimmed} |"))
}

fn unescaped_pipe_count(text: &str) -> usize {
    let mut count = 0usize;
    let mut previous_was_backslash = false;
    for ch in text.chars() {
        if ch == '|' && !previous_was_backslash {
            count += 1;
        }
        previous_was_backslash = ch == '\\' && !previous_was_backslash;
        if ch != '\\' {
            previous_was_backslash = false;
        }
    }
    count
}

pub fn byte_offset_for_line_column(text: &str, target_line: usize, target_column: usize) -> usize {
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

fn thematic_break_enter_transform(text: &str, cursor_offset: usize) -> SemanticEnterTransform {
    let cursor_offset = clamp_to_char_boundary(text, cursor_offset);
    if cursor_offset < text.len() {
        return split_block_transform(text, cursor_offset);
    }
    SemanticEnterTransform {
        replacement: format!("{text}\n\n"),
        cursor_offset: text.len() + 2,
    }
}

fn toc_enter_transform(text: &str, cursor_offset: usize) -> SemanticEnterTransform {
    structural_block_enter_transform(text, cursor_offset)
}

fn structural_block_enter_transform(text: &str, cursor_offset: usize) -> SemanticEnterTransform {
    let cursor_offset = clamp_to_char_boundary(text, cursor_offset);
    if cursor_offset < text.len() {
        return split_block_transform(text, cursor_offset);
    }
    SemanticEnterTransform {
        replacement: format!("{text}\n\n"),
        cursor_offset: text.len() + 2,
    }
}

fn math_block_enter_transform(text: &str, cursor_offset: usize) -> Option<SemanticEnterTransform> {
    let cursor_offset = clamp_to_char_boundary(text, cursor_offset);
    if cursor_offset < text.len() {
        return None;
    }

    Some(SemanticEnterTransform {
        replacement: format!("{text}\n\n"),
        cursor_offset: text.len() + 2,
    })
}

fn heading_enter_transform(text: &str, cursor_offset: usize) -> SemanticEnterTransform {
    if let Some((content, marker)) = setext_heading_parts(text) {
        if content.trim().is_empty() {
            return SemanticEnterTransform {
                replacement: String::new(),
                cursor_offset: 0,
            };
        }
        let cursor_offset = clamp_to_char_boundary(text, cursor_offset).min(content.len());
        let before = &content[..cursor_offset];
        let after = &content[cursor_offset..];
        let replacement = format!("{before}\n{marker}\n\n{after}");
        let cursor_offset = before.len() + 1 + marker.len() + 2;
        return SemanticEnterTransform {
            replacement,
            cursor_offset,
        };
    }

    // Find the heading marker length: "## " -> 3
    let trimmed = text.trim_start();
    let marker_len = trimmed.chars().take_while(|ch| *ch == '#').count();
    // The content after "## " (or "##" with no space)
    let content_start = if trimmed.get(marker_len..marker_len + 1) == Some(" ") {
        marker_len + 1
    } else {
        marker_len
    };
    let content = trimmed.get(content_start..).unwrap_or("").trim();

    if content.is_empty() {
        // Empty heading: exit heading, leave an empty paragraph
        SemanticEnterTransform {
            replacement: String::new(),
            cursor_offset: 0,
        }
    } else {
        split_block_transform(text, cursor_offset)
    }
}

fn setext_heading_parts(text: &str) -> Option<(&str, &str)> {
    let mut lines = text.lines();
    let content = lines.next()?;
    let marker = lines.next()?;
    if lines.next().is_some() {
        return None;
    }

    let marker = marker.trim();
    if !marker.is_empty()
        && marker
            .chars()
            .all(|ch| ch == '=' || ch == '-')
        && marker.chars().all(|ch| ch == marker.chars().next().unwrap())
    {
        Some((content, marker))
    } else {
        None
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
        if info.is_task && line_start == 0 && line_end == text.len() {
            let current_line = if info.marker_needs_space {
                format!("{line} ")
            } else {
                line.to_string()
            };
            let replacement = format!("{current_line}\n{}", info.continuation_prefix);
            return Some(SemanticEnterTransform {
                cursor_offset: replacement.len(),
                replacement,
            });
        }
        if info.indent_len > 0 {
            return Some(exit_indented_empty_list_line(
                text,
                line_start,
                line_end,
                info.indent_len,
            ));
        }
        return Some(exit_structured_line(text, line_start, line_end));
    }

    let split_offset = cursor_offset.max(line_start + info.current_prefix_end);
    let local_split = split_offset - line_start;
    let current_line = &line[..local_split];
    let moved_suffix = &line[local_split..];
    let before = &text[..line_start];
    let after = if let Some(marker) = info.ordered_marker {
        renumber_ordered_list_tail(
            &text[line_end..],
            &line[..info.indent_len],
            marker.delimiter,
            marker.number.saturating_add(2),
        )
    } else {
        text[line_end..].to_string()
    };
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

fn code_fence_enter_transform(text: &str, cursor_offset: usize) -> Option<SemanticEnterTransform> {
    let cursor_offset = clamp_to_char_boundary(text, cursor_offset);
    let (line_start, line_end) = line_bounds(text, cursor_offset);
    let line = &text[line_start..line_end];
    let trimmed = line.trim();
    if line_start == 0
        && line_end == text.len()
        && cursor_offset == line_end
        && let Some(marker) = opening_fence_marker(trimmed)
    {
        let replacement = format!("{line}\n\n{marker}");
        return Some(SemanticEnterTransform {
            cursor_offset: line.len() + 1,
            replacement,
        });
    }
    if !is_closing_fence(trimmed) {
        return None;
    }
    Some(exit_structured_line(text, line_start, line_end))
}

pub(crate) fn opening_fence_marker(s: &str) -> Option<String> {
    let marker_char = s.chars().next()?;
    if marker_char != '`' && marker_char != '~' {
        return None;
    }
    let marker_len = s.chars().take_while(|ch| *ch == marker_char).count();
    if marker_len < 3 {
        return None;
    }
    let info = &s[marker_len..];
    if info.chars().any(|ch| ch == marker_char) {
        return None;
    }
    Some(marker_char.to_string().repeat(marker_len))
}

fn is_closing_fence(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let backtick_count = s.chars().take_while(|c| *c == '`').count();
    if backtick_count >= 3 && backtick_count == s.len() {
        return true;
    }
    let tilde_count = s.chars().take_while(|c| *c == '~').count();
    tilde_count >= 3 && tilde_count == s.len()
}

fn blockquote_enter_transform(text: &str, cursor_offset: usize) -> Option<SemanticEnterTransform> {
    let cursor_offset = clamp_to_char_boundary(text, cursor_offset);
    let (line_start, line_end) = line_bounds(text, cursor_offset);
    let line = &text[line_start..line_end];
    let info = parse_blockquote_line(line)?;

    if let Some(list_info) = parse_list_line(&line[info.current_prefix_end..]) {
        if list_info.is_empty {
            let before = &text[..line_start];
            let after = &text[line_end..];
            let replacement = format!("{before}{}{after}", info.continuation_prefix);
            let cursor_offset = before.len() + info.continuation_prefix.len();
            return Some(SemanticEnterTransform {
                replacement,
                cursor_offset,
            });
        }

        let split_offset = cursor_offset.max(
            line_start + info.current_prefix_end + list_info.current_prefix_end,
        );
        let local_split = split_offset - line_start;
        let current_line = &line[..local_split];
        let moved_suffix = &line[local_split..];
        let before = &text[..line_start];
        let after = if let Some(marker) = list_info.ordered_marker {
            let nested_indent = &line[info.current_prefix_end
                ..info.current_prefix_end + list_info.indent_len];
            renumber_prefixed_ordered_list_tail(
                &text[line_end..],
                &info.continuation_prefix,
                nested_indent,
                marker.delimiter,
                marker.number.saturating_add(2),
            )
        } else {
            text[line_end..].to_string()
        };
        let replacement = format!(
            "{before}{current_line}\n{}{}{}{after}",
            info.continuation_prefix, list_info.continuation_prefix, moved_suffix
        );
        let cursor_offset = before.len()
            + current_line.len()
            + 1
            + info.continuation_prefix.len()
            + list_info.continuation_prefix.len();

        return Some(SemanticEnterTransform {
            replacement,
            cursor_offset,
        });
    }

    if info.is_empty {
        if line_start == 0 && line_end == text.len() {
            let replacement = format!("{}\n\n", info.continuation_prefix);
            let cursor_offset = info.continuation_prefix.len() + 1;
            return Some(SemanticEnterTransform {
                replacement,
                cursor_offset,
            });
        }
        return Some(exit_structured_line(text, line_start, line_end));
    }

    let split_offset = cursor_offset.max(line_start + info.current_prefix_end);
    let local_split = split_offset - line_start;
    let mut current_line = line[..local_split].to_string();
    if info.current_prefix_end == 1
        && line.as_bytes().get(1).is_some_and(|byte| !byte.is_ascii_whitespace())
    {
        current_line.insert(1, ' ');
    }
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

fn exit_indented_empty_list_line(
    text: &str,
    line_start: usize,
    line_end: usize,
    indent_len: usize,
) -> SemanticEnterTransform {
    let before = trim_trailing_newlines(&text[..line_start]);
    let after = trim_leading_newlines(&text[line_end..]);
    let indent_end = (line_start + indent_len).min(line_end);
    let indent = &text[line_start..indent_end];

    let (replacement, cursor_offset) = match (before.is_empty(), after.is_empty()) {
        (true, true) => (indent.to_string(), indent.len()),
        (true, false) => (format!("{indent}\n\n{after}"), indent.len()),
        (false, true) => {
            let replacement = format!("{before}\n\n{indent}");
            let cursor_offset = before.len() + 2 + indent.len();
            (replacement, cursor_offset)
        }
        (false, false) => {
            let replacement = format!("{before}\n\n{indent}\n\n{after}");
            let cursor_offset = before.len() + 2 + indent.len();
            (replacement, cursor_offset)
        }
    };

    SemanticEnterTransform {
        replacement,
        cursor_offset,
    }
}

fn renumber_ordered_list_tail(
    tail: &str,
    indent: &str,
    delimiter: char,
    first_number: usize,
) -> String {
    if tail.is_empty() {
        return String::new();
    }

    let mut next_number = first_number;
    let mut rebuilt = String::with_capacity(tail.len());
    let mut offset = 0usize;
    for segment in split_inclusive_lines(tail) {
        let line = segment.trim_end_matches(['\r', '\n']);
        let line_ending = &segment[line.len()..];
        if offset == 0 && line.is_empty() {
            rebuilt.push_str(segment);
            offset += segment.len();
            continue;
        }
        let Some(rest) = line.strip_prefix(indent) else {
            if line_indents_deeper_than(line, indent) {
                rebuilt.push_str(segment);
                offset += segment.len();
                continue;
            }
            rebuilt.push_str(&tail[offset..]);
            return rebuilt;
        };
        let Some(marker) = ordered_list_marker(rest) else {
            if rest_indents_deeper_than(rest, "") {
                rebuilt.push_str(segment);
                offset += segment.len();
                continue;
            }
            rebuilt.push_str(&tail[offset..]);
            return rebuilt;
        };
        if marker.delimiter != delimiter {
            rebuilt.push_str(&tail[offset..]);
            return rebuilt;
        }

        rebuilt.push_str(indent);
        rebuilt.push_str(&next_number.to_string());
        rebuilt.push(delimiter);
        rebuilt.push(' ');
        rebuilt.push_str(&rest[marker.prefix_len..]);
        rebuilt.push_str(line_ending);
        next_number = next_number.saturating_add(1);
        offset += segment.len();
    }

    rebuilt
}

fn renumber_prefixed_ordered_list_tail(
    tail: &str,
    line_prefix: &str,
    list_indent: &str,
    delimiter: char,
    first_number: usize,
) -> String {
    if tail.is_empty() {
        return String::new();
    }

    let mut next_number = first_number;
    let mut rebuilt = String::with_capacity(tail.len());
    let mut offset = 0usize;
    for segment in split_inclusive_lines(tail) {
        let line = segment.trim_end_matches(['\r', '\n']);
        let line_ending = &segment[line.len()..];
        if offset == 0 && line.is_empty() {
            rebuilt.push_str(segment);
            offset += segment.len();
            continue;
        }
        let Some(rest) = line.strip_prefix(line_prefix) else {
            rebuilt.push_str(&tail[offset..]);
            return rebuilt;
        };
        let Some(rest) = rest.strip_prefix(list_indent) else {
            if rest_indents_deeper_than(rest, list_indent) {
                rebuilt.push_str(segment);
                offset += segment.len();
                continue;
            }
            rebuilt.push_str(&tail[offset..]);
            return rebuilt;
        };
        let Some(marker) = ordered_list_marker(rest) else {
            if rest_indents_deeper_than(rest, "") {
                rebuilt.push_str(segment);
                offset += segment.len();
                continue;
            }
            rebuilt.push_str(&tail[offset..]);
            return rebuilt;
        };
        if marker.delimiter != delimiter {
            rebuilt.push_str(&tail[offset..]);
            return rebuilt;
        }

        rebuilt.push_str(line_prefix);
        rebuilt.push_str(list_indent);
        rebuilt.push_str(&next_number.to_string());
        rebuilt.push(delimiter);
        rebuilt.push(' ');
        rebuilt.push_str(&rest[marker.prefix_len..]);
        rebuilt.push_str(line_ending);
        next_number = next_number.saturating_add(1);
        offset += segment.len();
    }

    rebuilt
}

fn line_indents_deeper_than(line: &str, indent: &str) -> bool {
    let line_indent = leading_horizontal_whitespace(line);
    line_indent.len() > indent.len() && line_indent.starts_with(indent)
}

fn rest_indents_deeper_than(rest: &str, indent: &str) -> bool {
    let rest_indent = leading_horizontal_whitespace(rest);
    rest_indent.len() > indent.len() && rest_indent.starts_with(indent)
}

fn leading_horizontal_whitespace(text: &str) -> &str {
    let len = text
        .bytes()
        .take_while(|byte| matches!(byte, b' ' | b'\t'))
        .count();
    &text[..len]
}

fn split_inclusive_lines(text: &str) -> Vec<&str> {
    if text.is_empty() {
        return Vec::new();
    }

    let mut lines = Vec::new();
    let mut start = 0usize;
    for (index, ch) in text.char_indices() {
        if ch == '\n' {
            lines.push(&text[start..index + 1]);
            start = index + 1;
        }
    }
    if start < text.len() {
        lines.push(&text[start..]);
    }
    lines
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

fn pipe_table_delimiter_row(columns: usize) -> String {
    format!("| {} |", vec!["---"; columns].join(" | "))
}

fn pipe_table_empty_row(columns: usize) -> String {
    format!("| {} |", vec![""; columns].join(" | "))
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
                indent_len: indent_end,
                current_prefix_end,
                continuation_prefix,
                ordered_marker: None,
                is_empty,
                is_task: true,
                marker_needs_space: false,
            });
        }
    }

    for marker in ["- [ ]", "* [ ]", "+ [ ]", "- [x]", "* [x]", "+ [x]", "- [X]", "* [X]", "+ [X]"] {
        if trimmed == marker {
            let bullet = &marker[..1];
            let current_prefix_end = indent_end + marker.len();
            let continuation_prefix = format!("{indent}{bullet} [ ] ");
            return Some(ListLineInfo {
                indent_len: indent_end,
                current_prefix_end,
                continuation_prefix,
                ordered_marker: None,
                is_empty: true,
                is_task: true,
                marker_needs_space: true,
            });
        }
    }

    for marker in ["- ", "* ", "+ "] {
        if trimmed.starts_with(marker) {
            let current_prefix_end = indent_end + marker.len();
            let continuation_prefix = format!("{indent}{marker}");
            let is_empty = line[current_prefix_end..].trim().is_empty();
            return Some(ListLineInfo {
                indent_len: indent_end,
                current_prefix_end,
                continuation_prefix,
                ordered_marker: None,
                is_empty,
                is_task: false,
                marker_needs_space: false,
            });
        }
    }

    if let Some(marker) = ordered_list_marker(trimmed) {
        let current_prefix_end = indent_end + marker.prefix_len;
        let next_number = marker.number.saturating_add(1);
        let continuation_prefix = format!("{indent}{next_number}{} ", marker.delimiter);
        let is_empty = line[current_prefix_end..].trim().is_empty();
        return Some(ListLineInfo {
            indent_len: indent_end,
            current_prefix_end,
            continuation_prefix,
            ordered_marker: Some(marker),
            is_empty,
            is_task: false,
            marker_needs_space: false,
        });
    }

    None
}

fn ordered_list_marker(line_start: &str) -> Option<OrderedListMarker> {
    let bytes = line_start.as_bytes();
    let mut ix = 0usize;
    while ix < bytes.len() && bytes[ix].is_ascii_digit() {
        ix += 1;
    }

    if ix == 0 || ix + 1 >= bytes.len() {
        return None;
    }

    let delimiter = bytes[ix];
    if delimiter != b'.' && delimiter != b')' {
        return None;
    }
    if !bytes[ix + 1].is_ascii_whitespace() {
        return None;
    }

    let number = line_start[..ix].parse::<usize>().ok()?;
    Some(OrderedListMarker {
        number,
        delimiter: delimiter as char,
        prefix_len: ix + 2,
    })
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

fn quoted_content_prefix_end(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut ix = 0usize;
    while ix < bytes.len() && matches!(bytes[ix], b' ' | b'\t') {
        ix += 1;
    }

    let mut saw_marker = false;
    while ix < bytes.len() && bytes[ix] == b'>' {
        saw_marker = true;
        ix += 1;
        if ix < bytes.len() && bytes[ix] == b' ' {
            ix += 1;
        }
    }

    saw_marker.then_some(ix)
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
    fn set_heading_markup_sets_target_depth() {
        assert_eq!(set_heading_markup("Title", 1), "# Title");
        assert_eq!(set_heading_markup("Title", 2), "## Title");
        assert_eq!(set_heading_markup("# Title", 2), "## Title");
        assert_eq!(set_heading_markup("## Title", 1), "# Title");
        assert_eq!(set_heading_markup("## Title", 0), "Title");
        assert_eq!(set_heading_markup("# Title", 0), "Title");
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
    fn exits_empty_heading_on_enter_producing_empty_paragraph() {
        // "# " (marker only, no content) → empty paragraph
        let transform =
            semantic_enter_transform(&BlockKind::Heading { depth: 1 }, "# ", None, 2).unwrap();

        assert_eq!(transform.replacement, "");
        assert_eq!(transform.cursor_offset, 0);
    }

    #[test]
    fn exits_empty_heading_without_trailing_space() {
        // "##" (no space, no content) → empty paragraph
        let transform =
            semantic_enter_transform(&BlockKind::Heading { depth: 2 }, "##", None, 2).unwrap();

        assert_eq!(transform.replacement, "");
        assert_eq!(transform.cursor_offset, 0);
    }

    #[test]
    fn nonempty_heading_enter_still_splits() {
        // Non-empty heading should continue to split, same as before
        let transform =
            semantic_enter_transform(&BlockKind::Heading { depth: 2 }, "## A", None, 4).unwrap();

        assert_eq!(transform.replacement, "## A\n\n");
        assert_eq!(transform.cursor_offset, 6);
    }

    #[test]
    fn setext_heading_enter_splits_without_erasing_content() {
        let transform = semantic_enter_transform(
            &BlockKind::Heading { depth: 2 },
            "Title\n---",
            None,
            "Title".len(),
        )
        .unwrap();

        assert_eq!(transform.replacement, "Title\n---\n\n");
        assert_eq!(transform.cursor_offset, "Title\n---\n\n".len());
    }

    #[test]
    fn setext_heading_enter_moves_remainder_to_following_paragraph() {
        let transform = semantic_enter_transform(
            &BlockKind::Heading { depth: 1 },
            "Hello world\n=",
            None,
            "Hello".len(),
        )
        .unwrap();

        assert_eq!(transform.replacement, "Hello\n=\n\n world");
        assert_eq!(transform.cursor_offset, "Hello\n=\n\n".len());
    }

    #[test]
    fn empty_setext_heading_enter_exits_heading() {
        let transform =
            semantic_enter_transform(&BlockKind::Heading { depth: 2 }, "\n---", None, "\n---".len())
                .unwrap();

        assert_eq!(transform.replacement, "");
        assert_eq!(transform.cursor_offset, 0);
    }

    #[test]
    fn continues_list_item_and_moves_remainder() {
        let transform = semantic_enter_transform(&BlockKind::List, "- item", None, 3).unwrap();

        assert_eq!(transform.replacement, "- i\n- tem");
        assert_eq!(transform.cursor_offset, 6);
    }

    #[test]
    fn continues_parenthesized_ordered_list_item() {
        let transform = semantic_enter_transform(&BlockKind::List, "1) item", None, 7).unwrap();

        assert_eq!(transform.replacement, "1) item\n2) ");
        assert_eq!(transform.cursor_offset, "1) item\n2) ".len());
    }

    #[test]
    fn ordered_list_enter_renumbers_following_sibling_items() {
        let transform = semantic_enter_transform(
            &BlockKind::List,
            "1. one\n2. two\n3. three",
            None,
            "1. one".len(),
        )
        .unwrap();

        assert_eq!(transform.replacement, "1. one\n2. \n3. two\n4. three");
        assert_eq!(transform.cursor_offset, "1. one\n2. ".len());
    }

    #[test]
    fn ordered_list_enter_skips_nested_items_while_renumbering_siblings() {
        let transform = semantic_enter_transform(
            &BlockKind::List,
            "1. one\n2. two\n   1. child\n3. three",
            None,
            "1. one".len(),
        )
        .unwrap();

        assert_eq!(
            transform.replacement,
            "1. one\n2. \n3. two\n   1. child\n4. three"
        );
        assert_eq!(transform.cursor_offset, "1. one\n2. ".len());
    }

    #[test]
    fn exits_empty_task_list_item() {
        let transform =
            semantic_enter_transform(&BlockKind::List, "- one\n- [ ] ", None, 11).unwrap();

        assert_eq!(transform.replacement, "- one\n\n");
        assert_eq!(transform.cursor_offset, 7);
    }

    #[test]
    fn exits_indented_empty_list_item_after_indent() {
        let transform = semantic_enter_transform(&BlockKind::List, "  - ", None, 4).unwrap();

        assert_eq!(transform.replacement, "  ");
        assert_eq!(transform.cursor_offset, 2);
    }

    #[test]
    fn bare_task_marker_enter_creates_editable_task_item() {
        let transform = semantic_enter_transform(&BlockKind::List, "- [ ]", None, 5).unwrap();

        assert_eq!(transform.replacement, "- [ ] \n- [ ] ");
        assert_eq!(transform.cursor_offset, "- [ ] \n- [ ] ".len());
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
    fn continues_list_inside_blockquote() {
        let transform =
            semantic_enter_transform(&BlockKind::Blockquote, "> - item", None, 5).unwrap();

        assert_eq!(transform.replacement, "> - i\n> - tem");
        assert_eq!(transform.cursor_offset, 10);
    }

    #[test]
    fn continues_ordered_list_inside_callout() {
        let transform = semantic_enter_transform(
            &BlockKind::Callout {
                kind: "note".to_string(),
            },
            "> 1. first",
            None,
            "> 1. first".len(),
        )
        .unwrap();

        assert_eq!(transform.replacement, "> 1. first\n> 2. ");
        assert_eq!(transform.cursor_offset, "> 1. first\n> 2. ".len());
    }

    #[test]
    fn ordered_list_enter_inside_callout_renumbers_following_siblings() {
        let transform = semantic_enter_transform(
            &BlockKind::Callout {
                kind: "note".to_string(),
            },
            "> 1. first\n> 2. second\n> 3. third",
            None,
            "> 1. first".len(),
        )
        .unwrap();

        assert_eq!(
            transform.replacement,
            "> 1. first\n> 2. \n> 3. second\n> 4. third"
        );
        assert_eq!(transform.cursor_offset, "> 1. first\n> 2. ".len());
    }

    #[test]
    fn ordered_list_enter_inside_callout_skips_nested_items() {
        let transform = semantic_enter_transform(
            &BlockKind::Callout {
                kind: "note".to_string(),
            },
            "> 1. first\n> 2. second\n>    1. child\n> 3. third",
            None,
            "> 1. first".len(),
        )
        .unwrap();

        assert_eq!(
            transform.replacement,
            "> 1. first\n> 2. \n> 3. second\n>    1. child\n> 4. third"
        );
        assert_eq!(transform.cursor_offset, "> 1. first\n> 2. ".len());
    }

    #[test]
    fn continues_task_list_inside_blockquote_as_unchecked_item() {
        let transform = semantic_enter_transform(
            &BlockKind::Blockquote,
            "> - [x] done",
            None,
            "> - [x] done".len(),
        )
        .unwrap();

        assert_eq!(transform.replacement, "> - [x] done\n> - [ ] ");
        assert_eq!(transform.cursor_offset, "> - [x] done\n> - [ ] ".len());
    }

    #[test]
    fn exits_empty_list_item_inside_blockquote_to_quote_line() {
        let transform =
            semantic_enter_transform(&BlockKind::Blockquote, "> keep\n> - ", None, 11).unwrap();

        assert_eq!(transform.replacement, "> keep\n> ");
        assert_eq!(transform.cursor_offset, 9);
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
        assert!(supports_semantic_enter(&BlockKind::CodeFence {
            language: Some("rust".to_string()),
        }));
        assert!(!supports_semantic_enter(&BlockKind::Table));
    }

    #[test]
    fn pipe_table_enter_builds_typora_style_table() {
        let transform =
            pipe_table_enter_transform(&BlockKind::Paragraph, "| a | b |", None, 9).unwrap();

        assert_eq!(transform.replacement, "| a | b |\n| --- | --- |\n|  |  |");
        assert_eq!(transform.cursor_offset, "| a | b |\n| --- | --- |\n|".len());
    }

    #[test]
    fn pipe_table_enter_accepts_header_without_outer_pipes() {
        let transform = pipe_table_enter_transform(&BlockKind::Paragraph, "a | b", None, 5).unwrap();

        assert_eq!(transform.replacement, "| a | b |\n| --- | --- |\n|  |  |");
        assert_eq!(transform.cursor_offset, "| a | b |\n| --- | --- |\n|".len());
    }

    #[test]
    fn pipe_table_enter_does_not_trigger_for_invalid_shapes() {
        assert!(pipe_table_enter_transform(&BlockKind::Paragraph, "| a |", None, 5).is_none());
        assert!(pipe_table_enter_transform(&BlockKind::Paragraph, "a \\| b", None, 6).is_none());
        assert!(pipe_table_enter_transform(&BlockKind::Paragraph, "| a | b ", None, 8).is_none());
        assert!(
            pipe_table_enter_transform(&BlockKind::Paragraph, "| a |\n| b |", None, 11).is_none()
        );
        assert!(
            pipe_table_enter_transform(&BlockKind::Paragraph, "| a | b |", Some(0..1), 9).is_none()
        );
        assert!(pipe_table_enter_transform(&BlockKind::Paragraph, "| a | b |", None, 4).is_none());
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

    #[test]
    fn adjusts_plus_task_list_markup() {
        assert_eq!(
            adjust_block_markup("+ [X] done", true),
            Some("  + [X] done".to_string())
        );
        assert_eq!(
            adjust_block_markup("  + [X] done", false),
            Some("+ [X] done".to_string())
        );
    }

    #[test]
    fn indents_list_inside_blockquote_without_deepening_quote() {
        let transform = adjust_quoted_list_markup_at_cursor("> - item", "> - item".len(), true)
            .expect("quoted list adjustment");

        assert_eq!(transform.replacement, ">   - item");
        assert_eq!(transform.cursor_offset, ">   - item".len());
    }

    #[test]
    fn outdents_list_inside_blockquote() {
        let transform = adjust_quoted_list_markup_at_cursor(">   - item", ">   - item".len(), false)
            .expect("quoted list adjustment");

        assert_eq!(transform.replacement, "> - item");
        assert_eq!(transform.cursor_offset, "> - item".len());
    }

    #[test]
    fn outdents_top_level_quoted_list_line_to_quote_text() {
        let transform = adjust_quoted_list_markup_at_cursor("> - item", "> - item".len(), false)
            .expect("quoted list adjustment");

        assert_eq!(transform.replacement, "> item");
        assert_eq!(transform.cursor_offset, "> item".len());
    }

    #[test]
    fn indents_current_list_line() {
        let transform = adjust_list_markup_at_cursor("- one\n- two", "- one\n- two".len(), true)
            .expect("list adjustment");

        assert_eq!(transform.replacement, "- one\n  - two");
        assert_eq!(transform.cursor_offset, "- one\n  - two".len());
    }

    #[test]
    fn outdents_current_list_line() {
        let transform = adjust_list_markup_at_cursor("- one\n  - two", "- one\n  - two".len(), false)
            .expect("list adjustment");

        assert_eq!(transform.replacement, "- one\n- two");
        assert_eq!(transform.cursor_offset, "- one\n- two".len());
    }

    #[test]
    fn outdents_top_level_list_line_to_plain_text() {
        let transform = adjust_list_markup_at_cursor("- one", "- one".len(), false)
            .expect("list adjustment");

        assert_eq!(transform.replacement, "one");
        assert_eq!(transform.cursor_offset, "one".len());
    }

    #[test]
    fn indents_selected_list_lines() {
        let transform = adjust_selected_list_markup("- one\n- two\n- three", 0..11, true)
            .expect("selected list adjustment");

        assert_eq!(transform.replacement, "  - one\n  - two\n- three");
        assert_eq!(transform.selection, 0..15);
    }

    #[test]
    fn outdents_selected_top_level_list_lines_to_plain_text() {
        let transform = adjust_selected_list_markup("- one\n- two\n- three", 0..11, false)
            .expect("selected list adjustment");

        assert_eq!(transform.replacement, "one\ntwo\n- three");
        assert_eq!(transform.selection, 0..7);
    }

    #[test]
    fn indents_selected_quoted_list_lines() {
        let source = "> - one\n> - two\n> tail";
        let transform = adjust_selected_quoted_list_markup(source, 0..15, true)
            .expect("selected quoted list adjustment");

        assert_eq!(transform.replacement, ">   - one\n>   - two\n> tail");
        assert_eq!(transform.selection, 0..19);
    }

    #[test]
    fn outdents_selected_top_level_quoted_list_lines_to_quote_text() {
        let source = "> - one\n> - two\n> tail";
        let transform = adjust_selected_quoted_list_markup(source, 0..15, false)
            .expect("selected quoted list adjustment");

        assert_eq!(transform.replacement, "> one\n> two\n> tail");
        assert_eq!(transform.selection, 0..11);
    }

    #[test]
    fn set_blockquote_markup_wraps_and_strips_prefix() {
        assert_eq!(set_blockquote_markup("Hello", true), "> Hello");
        assert_eq!(set_blockquote_markup("> Hello", false), "Hello");
        assert_eq!(set_blockquote_markup(">Hello", false), "Hello");
        // multiline
        assert_eq!(
            set_blockquote_markup("line1\nline2", true),
            "> line1\n> line2"
        );
        assert_eq!(
            set_blockquote_markup("> line1\n> line2", false),
            "line1\nline2"
        );
    }

    #[test]
    fn set_list_markup_converts_paragraph_to_bullet() {
        assert_eq!(set_list_markup("Hello", false), "- Hello");
    }

    #[test]
    fn set_list_markup_converts_paragraph_to_ordered() {
        assert_eq!(set_list_markup("Hello", true), "1. Hello");
    }

    #[test]
    fn set_list_markup_strips_bullet_on_toggle() {
        assert_eq!(set_list_markup("- Hello", false), "Hello");
    }

    #[test]
    fn set_list_markup_strips_ordered_on_toggle() {
        assert_eq!(set_list_markup("1. Hello", true), "Hello");
        assert_eq!(set_list_markup("1) Hello", true), "Hello");
    }

    #[test]
    fn set_list_markup_converts_ordered_to_bullet() {
        assert_eq!(set_list_markup("1. Hello", false), "- Hello");
        assert_eq!(set_list_markup("1) Hello", false), "- Hello");
    }

    #[test]
    fn set_list_markup_converts_bullet_to_ordered() {
        assert_eq!(set_list_markup("- Hello", true), "1. Hello");
    }

    #[test]
    fn set_list_markup_strips_plus_task_markers() {
        assert_eq!(set_list_markup("+ [X] Done", false), "Done");
        assert_eq!(set_list_markup("+ [X] Done", true), "1. Done");
    }

    #[test]
    fn set_task_list_markup_converts_typed_marker_to_task() {
        assert_eq!(set_task_list_markup("- [ ] task"), "- [ ] task");
        assert_eq!(set_task_list_markup("- [ ]"), "- [ ] ");
        assert_eq!(set_task_list_markup("* [x] done"), "- [x] done");
        assert_eq!(set_task_list_markup("+ [X] done"), "- [x] done");
        assert_eq!(set_task_list_markup("+ [X]"), "- [x] ");
        assert_eq!(set_task_list_markup("+ todo"), "- [ ] todo");
    }

    #[test]
    fn set_task_list_markup_converts_each_line() {
        assert_eq!(
            set_task_list_markup("one\n* two\n3. three"),
            "- [ ] one\n- [ ] two\n- [ ] three"
        );
    }

    #[test]
    fn detect_auto_format_heading() {
        assert_eq!(
            detect_auto_format(&BlockKind::Paragraph, "# Hello"),
            Some(AutoFormatAction::Heading { depth: 1 })
        );
        assert_eq!(
            detect_auto_format(&BlockKind::Paragraph, "## Hello"),
            Some(AutoFormatAction::Heading { depth: 2 })
        );
        assert_eq!(
            detect_auto_format(&BlockKind::Paragraph, "### Hello"),
            Some(AutoFormatAction::Heading { depth: 3 })
        );
        assert_eq!(
            detect_auto_format(&BlockKind::Paragraph, "###### Hello"),
            Some(AutoFormatAction::Heading { depth: 6 })
        );
    }

    #[test]
    fn detect_auto_format_blockquote() {
        assert_eq!(
            detect_auto_format(&BlockKind::Paragraph, "> Hello"),
            Some(AutoFormatAction::Blockquote)
        );
        assert_eq!(
            detect_auto_format(&BlockKind::Paragraph, ">"),
            Some(AutoFormatAction::Blockquote)
        );
        assert_eq!(
            detect_auto_format(&BlockKind::Paragraph, ">Hello"),
            Some(AutoFormatAction::Blockquote)
        );
    }

    #[test]
    fn detect_auto_format_bullet_list() {
        assert_eq!(
            detect_auto_format(&BlockKind::Paragraph, "- "),
            Some(AutoFormatAction::BulletList)
        );
        assert_eq!(
            detect_auto_format(&BlockKind::Paragraph, "* "),
            Some(AutoFormatAction::BulletList)
        );
    }

    #[test]
    fn detect_auto_format_ordered_list() {
        assert_eq!(
            detect_auto_format(&BlockKind::Paragraph, "1. Hello"),
            Some(AutoFormatAction::OrderedList)
        );
        assert_eq!(
            detect_auto_format(&BlockKind::Paragraph, "1) Hello"),
            Some(AutoFormatAction::OrderedList)
        );
    }

    #[test]
    fn detect_auto_format_task_list() {
        assert_eq!(
            detect_auto_format(&BlockKind::Paragraph, "- [ ] "),
            Some(AutoFormatAction::TaskList)
        );
        assert_eq!(
            detect_auto_format(&BlockKind::Paragraph, "* [x] done"),
            Some(AutoFormatAction::TaskList)
        );
        assert_eq!(
            detect_auto_format(&BlockKind::Paragraph, "+ [X] done"),
            Some(AutoFormatAction::TaskList)
        );
        assert_eq!(
            detect_auto_format(&BlockKind::Paragraph, "- [ ]"),
            Some(AutoFormatAction::TaskList)
        );
        assert_eq!(
            detect_auto_format(&BlockKind::Paragraph, "- [x]"),
            Some(AutoFormatAction::TaskList)
        );
        assert_eq!(detect_auto_format(&BlockKind::Paragraph, "- [ ]todo"), None);
    }

    #[test]
    fn detect_auto_format_horizontal_rule() {
        assert_eq!(
            detect_auto_format(&BlockKind::Paragraph, "---"),
            Some(AutoFormatAction::HorizontalRule)
        );
        assert_eq!(
            detect_auto_format(&BlockKind::Paragraph, "***"),
            Some(AutoFormatAction::HorizontalRule)
        );
        assert_eq!(
            detect_auto_format(&BlockKind::Paragraph, "___"),
            Some(AutoFormatAction::HorizontalRule)
        );
        assert_eq!(
            detect_auto_format(&BlockKind::Paragraph, "------"),
            Some(AutoFormatAction::HorizontalRule)
        );
    }

    #[test]
    fn detect_auto_format_code_fence() {
        assert_eq!(
            detect_auto_format(&BlockKind::Paragraph, "```"),
            Some(AutoFormatAction::CodeFence {
                marker: "```".to_string(),
                info: String::new(),
            })
        );
        assert_eq!(
            detect_auto_format(&BlockKind::Paragraph, "````"),
            Some(AutoFormatAction::CodeFence {
                marker: "````".to_string(),
                info: String::new(),
            })
        );
        assert_eq!(
            detect_auto_format(&BlockKind::Paragraph, "```rust"),
            Some(AutoFormatAction::CodeFence {
                marker: "```".to_string(),
                info: "rust".to_string(),
            })
        );
        assert_eq!(
            detect_auto_format(&BlockKind::Paragraph, "~~~ js"),
            Some(AutoFormatAction::CodeFence {
                marker: "~~~".to_string(),
                info: "js".to_string(),
            })
        );
    }

    #[test]
    fn detect_auto_format_returns_none_for_non_matching() {
        assert_eq!(detect_auto_format(&BlockKind::Paragraph, "Hello"), None);
        assert_eq!(detect_auto_format(&BlockKind::Paragraph, ""), None);
        assert_eq!(detect_auto_format(&BlockKind::Paragraph, "#"), None);
        assert_eq!(detect_auto_format(&BlockKind::Paragraph, "--"), None);
    }

    #[test]
    fn detect_auto_format_returns_none_for_non_paragraph() {
        assert_eq!(
            detect_auto_format(&BlockKind::Heading { depth: 1 }, "# Hello"),
            None
        );
        assert_eq!(detect_auto_format(&BlockKind::List, "- Hello"), None);
    }

    #[test]
    fn thematic_break_enter_creates_new_paragraph() {
        let transform =
            semantic_enter_transform(&BlockKind::ThematicBreak, "---", None, 3).unwrap();
        assert_eq!(transform.replacement, "---\n\n");
        assert_eq!(transform.cursor_offset, 5);
    }

    #[test]
    fn toc_enter_creates_following_paragraph() {
        let transform = semantic_enter_transform(&BlockKind::Toc, "[toc]", None, 5).unwrap();
        assert_eq!(transform.replacement, "[toc]\n\n");
        assert_eq!(transform.cursor_offset, 7);
    }

    #[test]
    fn footnote_definition_enter_creates_following_paragraph() {
        let transform =
            semantic_enter_transform(&BlockKind::FootnoteDefinition, "[^1]: Note", None, 10)
                .unwrap();
        assert_eq!(transform.replacement, "[^1]: Note\n\n");
        assert_eq!(transform.cursor_offset, 12);
    }

    #[test]
    fn link_reference_definition_enter_creates_following_paragraph() {
        let transform = semantic_enter_transform(
            &BlockKind::LinkReferenceDefinition,
            "[docs]: https://example.com",
            None,
            "[docs]: https://example.com".len(),
        )
        .unwrap();
        assert_eq!(transform.replacement, "[docs]: https://example.com\n\n");
        assert_eq!(transform.cursor_offset, "[docs]: https://example.com\n\n".len());
    }

    #[test]
    fn yaml_front_matter_enter_creates_following_paragraph() {
        let text = "---\ntitle: Draft\n---";
        let transform =
            semantic_enter_transform(&BlockKind::YamlFrontMatter, text, None, text.len()).unwrap();
        assert_eq!(transform.replacement, "---\ntitle: Draft\n---\n\n");
        assert_eq!(transform.cursor_offset, "---\ntitle: Draft\n---\n\n".len());
    }

    #[test]
    fn html_block_enter_creates_following_paragraph() {
        let text = "<div>Note</div>";
        let transform = semantic_enter_transform(&BlockKind::Html, text, None, text.len()).unwrap();
        assert_eq!(transform.replacement, "<div>Note</div>\n\n");
        assert_eq!(transform.cursor_offset, "<div>Note</div>\n\n".len());
    }

    #[test]
    fn math_block_enter_after_closing_delimiter_creates_following_paragraph() {
        let text = "$$\nx + y\n$$";
        let transform =
            semantic_enter_transform(&BlockKind::MathBlock, text, None, text.len()).unwrap();
        assert_eq!(transform.replacement, "$$\nx + y\n$$\n\n");
        assert_eq!(transform.cursor_offset, "$$\nx + y\n$$\n\n".len());
    }

    #[test]
    fn math_block_enter_inside_body_uses_plain_line_break() {
        let text = "$$\nx + y\n$$";
        assert!(semantic_enter_transform(&BlockKind::MathBlock, text, None, 6).is_none());
    }
}
