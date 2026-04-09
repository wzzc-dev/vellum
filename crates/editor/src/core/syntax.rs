use std::{cmp, ops::Range};

use ropey::Rope;
use tree_sitter::{InputEdit, Parser, Point, Tree};
use tree_sitter_md::LANGUAGE;

use super::document::{BlockKind, CursorAnchorPolicy};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BlockSeed {
    pub(crate) kind: BlockKind,
    pub(crate) content_range: Range<usize>,
    pub(crate) cursor_anchor_policy: CursorAnchorPolicy,
    pub(crate) can_code_edit: bool,
}

pub(crate) struct SyntaxState {
    parser: Parser,
    tree: Tree,
}

impl SyntaxState {
    pub(crate) fn from_source(source: &Rope) -> Self {
        let mut parser = new_parser();
        let tree = parse_rope(&mut parser, source, None)
            .expect("tree-sitter markdown block parse should succeed");
        Self { parser, tree }
    }

    pub(crate) fn reparse(&mut self, source: &Rope, edit: InputEdit) {
        self.tree.edit(&edit);
        self.tree = parse_rope(&mut self.parser, source, Some(&self.tree))
            .or_else(|| parse_rope(&mut self.parser, source, None))
            .expect("tree-sitter markdown incremental parse should succeed");
    }

    pub(crate) fn block_seeds(&self, source: &Rope) -> Vec<BlockSeed> {
        let mut seeds = Vec::new();
        collect_block_seeds(self.tree.root_node(), source, &mut seeds);
        seeds
    }
}

pub(crate) fn input_edit_for_splice(
    source: &Rope,
    range: Range<usize>,
    replacement: &str,
) -> InputEdit {
    let start = cmp::min(range.start, source.len_bytes());
    let old_end = cmp::min(range.end, source.len_bytes());
    let start_position = point_for_byte_offset(source, start);
    InputEdit {
        start_byte: start,
        old_end_byte: old_end,
        new_end_byte: start + replacement.len(),
        start_position,
        old_end_position: point_for_byte_offset(source, old_end),
        new_end_position: advance_point(start_position, replacement),
    }
}

fn new_parser() -> Parser {
    let mut parser = Parser::new();
    parser
        .set_language(&LANGUAGE.into())
        .expect("failed to load tree-sitter markdown block grammar");
    parser
}

fn parse_rope<'a>(parser: &mut Parser, source: &'a Rope, old_tree: Option<&Tree>) -> Option<Tree> {
    let len = source.len_bytes();
    parser.parse_with_options(
        &mut |byte, _| {
            let clamped = cmp::min(byte, len);
            let (chunk, chunk_start, _, _) = source.chunk_at_byte(clamped);
            &chunk[clamped - chunk_start..]
        },
        old_tree,
        None,
    )
}

fn collect_block_seeds<'tree>(
    node: tree_sitter::Node<'tree>,
    source: &Rope,
    seeds: &mut Vec<BlockSeed>,
) {
    if !node.is_named() {
        return;
    }

    match node.kind() {
        "document" | "section" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                collect_block_seeds(child, source, seeds);
            }
        }
        _ => {
            if let Some(seed) = block_seed_from_node(node, source) {
                seeds.push(seed);
            }
        }
    }
}

fn block_seed_from_node<'tree>(
    node: tree_sitter::Node<'tree>,
    source: &Rope,
) -> Option<BlockSeed> {
    let start_byte = node.start_byte();
    let content_end = content_end_for_node(source, node);
    let content_range = start_byte..content_end;
    let (kind, cursor_anchor_policy, can_code_edit) = match node.kind() {
        "paragraph" => (BlockKind::Paragraph, CursorAnchorPolicy::Clamp, false),
        "atx_heading" => (
            BlockKind::Heading {
                depth: atx_heading_depth(node),
            },
            CursorAnchorPolicy::Clamp,
            false,
        ),
        "setext_heading" => (
            BlockKind::Heading {
                depth: setext_heading_depth(node),
            },
            CursorAnchorPolicy::Clamp,
            false,
        ),
        "block_quote" => (BlockKind::Blockquote, CursorAnchorPolicy::Clamp, false),
        "list" => (BlockKind::List, CursorAnchorPolicy::Clamp, false),
        "pipe_table" => (BlockKind::Table, CursorAnchorPolicy::Clamp, false),
        "fenced_code_block" => (
            BlockKind::CodeFence {
                language: fenced_code_language(node, source),
            },
            CursorAnchorPolicy::PreserveColumn,
            true,
        ),
        "indented_code_block" => (
            BlockKind::CodeFence { language: None },
            CursorAnchorPolicy::PreserveColumn,
            true,
        ),
        "thematic_break" => (BlockKind::ThematicBreak, CursorAnchorPolicy::Clamp, false),
        "html_block" => (BlockKind::Html, CursorAnchorPolicy::Clamp, false),
        "minus_metadata" | "plus_metadata" => {
            (BlockKind::Raw, CursorAnchorPolicy::Clamp, false)
        }
        "link_reference_definition" => (BlockKind::Unknown, CursorAnchorPolicy::Clamp, false),
        _ => recover_block_shape(source, start_byte, node.end_byte())
            .unwrap_or((BlockKind::Unknown, CursorAnchorPolicy::Clamp, false)),
    };

    Some(BlockSeed {
        kind,
        content_range,
        cursor_anchor_policy,
        can_code_edit,
    })
}

fn atx_heading_depth<'tree>(node: tree_sitter::Node<'tree>) -> u8 {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        let depth = match child.kind() {
            "atx_h1_marker" => Some(1),
            "atx_h2_marker" => Some(2),
            "atx_h3_marker" => Some(3),
            "atx_h4_marker" => Some(4),
            "atx_h5_marker" => Some(5),
            "atx_h6_marker" => Some(6),
            _ => None,
        };
        if let Some(depth) = depth {
            return depth;
        }
    }

    1
}

fn setext_heading_depth<'tree>(node: tree_sitter::Node<'tree>) -> u8 {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "setext_h1_underline" => return 1,
            "setext_h2_underline" => return 2,
            _ => {}
        }
    }

    1
}

fn fenced_code_language<'tree>(node: tree_sitter::Node<'tree>, source: &Rope) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() != "info_string" {
            continue;
        }

        let mut inner_cursor = child.walk();
        for inner in child.named_children(&mut inner_cursor) {
            if inner.kind() == "language" {
                let text = source
                    .byte_slice(inner.start_byte()..inner.end_byte())
                    .to_string();
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }

        let text = source
            .byte_slice(child.start_byte()..child.end_byte())
            .to_string();
        let trimmed = text.trim();
        if let Some(language) = trimmed.split_whitespace().next().filter(|s| !s.is_empty()) {
            return Some(language.to_string());
        }
    }

    None
}

fn recover_block_shape(
    source: &Rope,
    start: usize,
    end: usize,
) -> Option<(BlockKind, CursorAnchorPolicy, bool)> {
    let text = source.byte_slice(start..end).to_string();
    if let Some(depth) = atx_heading_depth_from_text(&text) {
        return Some((BlockKind::Heading { depth }, CursorAnchorPolicy::Clamp, false));
    }

    if looks_like_list_block(&text) {
        return Some((BlockKind::List, CursorAnchorPolicy::Clamp, false));
    }

    if looks_like_blockquote_block(&text) {
        return Some((BlockKind::Blockquote, CursorAnchorPolicy::Clamp, false));
    }

    None
}

fn atx_heading_depth_from_text(text: &str) -> Option<u8> {
    let trimmed = text.trim_end_matches(['\r', '\n']);
    if trimmed.is_empty() || trimmed.contains('\n') {
        return None;
    }

    let line = strip_markdown_indent(trimmed);
    let depth = line.bytes().take_while(|byte| *byte == b'#').count();
    if !(1..=6).contains(&depth) {
        return None;
    }

    let rest = &line[depth..];
    if rest.is_empty() || matches!(rest.chars().next(), Some(' ' | '\t')) {
        Some(depth as u8)
    } else {
        None
    }
}

pub(crate) fn looks_like_list_block(text: &str) -> bool {
    let trimmed = text.trim_end_matches(['\r', '\n']);
    !trimmed.is_empty() && trimmed.lines().all(is_list_item_line)
}

fn is_list_item_line(line: &str) -> bool {
    let line = line.trim_end_matches('\r');
    if line.trim_matches([' ', '\t']).is_empty() {
        return true;
    }

    let line = strip_markdown_indent(line);
    if let Some(rest) = line
        .strip_prefix('-')
        .or_else(|| line.strip_prefix('*'))
        .or_else(|| line.strip_prefix('+'))
    {
        return rest.is_empty() || matches!(rest.chars().next(), Some(' ' | '\t'));
    }

    let digit_len = line.bytes().take_while(|byte| byte.is_ascii_digit()).count();
    if digit_len == 0 {
        return false;
    }

    let marker = line.as_bytes().get(digit_len).copied();
    if !matches!(marker, Some(b'.' | b')')) {
        return false;
    }

    let rest = &line[digit_len + 1..];
    rest.is_empty() || matches!(rest.chars().next(), Some(' ' | '\t'))
}

pub(crate) fn looks_like_blockquote_block(text: &str) -> bool {
    let trimmed = text.trim_end_matches(['\r', '\n']);
    !trimmed.is_empty() && trimmed.lines().all(is_blockquote_line)
}

fn is_blockquote_line(line: &str) -> bool {
    let line = line.trim_end_matches('\r');
    if line.trim_matches([' ', '\t']).is_empty() {
        return true;
    }

    strip_markdown_indent(line).starts_with('>')
}

fn strip_markdown_indent(line: &str) -> &str {
    let mut indent = 0usize;
    let mut offset = 0usize;
    for ch in line.chars() {
        if indent == 3 || ch != ' ' {
            break;
        }
        indent += 1;
        offset += ch.len_utf8();
    }

    &line[offset..]
}

fn point_for_byte_offset(source: &Rope, byte_offset: usize) -> Point {
    if source.len_bytes() == 0 {
        return Point { row: 0, column: 0 };
    }

    let clamped = cmp::min(byte_offset, source.len_bytes());
    let char_idx = source.byte_to_char(clamped);
    let row = source.char_to_line(char_idx);
    let line_start = source.line_to_byte(row);
    Point {
        row,
        column: clamped - line_start,
    }
}

fn advance_point(mut point: Point, text: &str) -> Point {
    if text.is_empty() {
        return point;
    }

    let mut newline_count = 0usize;
    let mut last_line_start = 0usize;
    for (idx, byte) in text.bytes().enumerate() {
        if byte == b'\n' {
            newline_count += 1;
            last_line_start = idx + 1;
        }
    }

    if newline_count == 0 {
        point.column += text.len();
    } else {
        point.row += newline_count;
        point.column = text.len() - last_line_start;
    }

    point
}

fn trimmed_content_end(source: &Rope, start: usize, end: usize) -> usize {
    let mut trimmed = end;
    while trimmed > start {
        match source.byte(trimmed - 1) {
            b'\n' | b'\r' => trimmed -= 1,
            _ => break,
        }
    }
    trimmed
}

fn content_end_for_node<'tree>(source: &Rope, node: tree_sitter::Node<'tree>) -> usize {
    let start = node.start_byte();
    let end = node.end_byte();
    let trimmed = trimmed_content_end(source, start, end);

    match node.kind() {
        "list" => restore_single_trailing_newline(source, trimmed, end),
        _ => trimmed,
    }
}

fn restore_single_trailing_newline(source: &Rope, trimmed_end: usize, raw_end: usize) -> usize {
    let mut offset = trimmed_end;
    while offset < raw_end {
        match source.byte(offset) {
            b'\n' => return offset + 1,
            b'\r' | b' ' | b'\t' => offset += 1,
            _ => break,
        }
    }

    trimmed_end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_block_kinds_from_tree_sitter_markdown() {
        let source = Rope::from_str(
            "# Heading\n\n- item\n\n> quote\n\n| a | b |\n| - | - |\n| 1 | 2 |\n\n```rust\nfn main() {}\n```\n",
        );
        let syntax = SyntaxState::from_source(&source);
        let seeds = syntax.block_seeds(&source);

        assert!(seeds.iter().any(|seed| matches!(seed.kind, BlockKind::Heading { depth: 1 })));
        assert!(seeds.iter().any(|seed| seed.kind == BlockKind::List));
        assert!(seeds.iter().any(|seed| seed.kind == BlockKind::Blockquote));
        assert!(seeds.iter().any(|seed| seed.kind == BlockKind::Table));
        assert!(seeds.iter().any(|seed| {
            matches!(
                seed.kind,
                BlockKind::CodeFence {
                    language: Some(ref language)
                } if language == "rust"
            )
        }));
    }

    #[test]
    fn computes_tree_sitter_edit_positions_in_bytes() {
        let source = Rope::from_str("a\n你好\n");
        let edit = input_edit_for_splice(&source, 2..5, "xyz");

        assert_eq!(edit.start_position, Point { row: 1, column: 0 });
        assert_eq!(edit.old_end_position, Point { row: 1, column: 3 });
        assert_eq!(edit.new_end_position, Point { row: 1, column: 3 });
    }
}
