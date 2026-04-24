use std::{cmp, collections::HashMap, ops::Range};

use ropey::Rope;
use tree_sitter::{InputEdit, Node, Parser, Point, Tree};
use tree_sitter_md::LANGUAGE;

use super::document::{BlockKind, CursorAnchorPolicy};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BlockSeed {
    pub(crate) kind: BlockKind,
    pub(crate) content_range: Range<usize>,
    pub(crate) cursor_anchor_policy: CursorAnchorPolicy,
    pub(crate) can_code_edit: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct InlineStyle {
    pub(crate) strong: bool,
    pub(crate) emphasis: bool,
    pub(crate) strikethrough: bool,
    pub(crate) code: bool,
    pub(crate) link: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InlineSegment {
    pub(crate) text: String,
    pub(crate) style: InlineStyle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PreviewListMarker {
    Bullet,
    Ordered(String),
    Task { checked: bool },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreviewListItem {
    pub(crate) marker: PreviewListMarker,
    pub(crate) blocks: Vec<PreviewBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PreviewBlock {
    Paragraph {
        content: Vec<InlineSegment>,
    },
    Heading {
        depth: u8,
        content: Vec<InlineSegment>,
    },
    List {
        items: Vec<PreviewListItem>,
    },
    Blockquote {
        blocks: Vec<PreviewBlock>,
    },
    Table {
        header: Vec<Vec<InlineSegment>>,
        rows: Vec<Vec<Vec<InlineSegment>>>,
    },
    CodeFence {
        language: Option<String>,
        text: String,
    },
    ThematicBreak,
    Html {
        text: String,
    },
    Raw {
        text: String,
    },
    Unknown {
        text: String,
    },
}

struct MarkdownParser {
    parser: Parser,
}

#[derive(Debug, Clone)]
struct MarkdownTree {
    block_tree: Tree,
}

impl MarkdownTree {
    fn edit(&mut self, edit: &InputEdit) {
        self.block_tree.edit(edit);
    }

    fn block_tree(&self) -> &Tree {
        &self.block_tree
    }
}

impl Default for MarkdownParser {
    fn default() -> Self {
        Self {
            parser: Parser::new(),
        }
    }
}

impl MarkdownParser {
    fn parse_with<'a, T, F>(
        &mut self,
        callback: &mut F,
        old_tree: Option<&MarkdownTree>,
    ) -> Option<MarkdownTree>
    where
        T: AsRef<[u8]>,
        F: FnMut(usize, Point) -> T,
    {
        self.parser
            .set_included_ranges(&[])
            .expect("failed to reset included ranges for block parse");
        self.parser
            .set_language(&LANGUAGE.into())
            .expect("failed to load markdown block grammar");
        let block_tree = self.parser.parse_with_options(
            callback,
            old_tree.map(|tree| &tree.block_tree),
            None,
        )?;

        Some(MarkdownTree { block_tree })
    }
}

pub(crate) struct SyntaxState {
    parser: MarkdownParser,
    tree: MarkdownTree,
    preview_by_content_range: HashMap<(usize, usize), PreviewBlock>,
}

impl SyntaxState {
    pub(crate) fn from_source(source: &Rope) -> Self {
        let mut parser = MarkdownParser::default();
        let tree = parse_rope(&mut parser, source, None)
            .expect("tree-sitter markdown parse should succeed");
        Self {
            parser,
            tree,
            preview_by_content_range: HashMap::new(),
        }
    }

    pub(crate) fn reparse(&mut self, source: &Rope, edit: InputEdit) {
        self.tree.edit(&edit);
        self.tree = parse_rope(&mut self.parser, source, Some(&self.tree))
            .or_else(|| parse_rope(&mut self.parser, source, None))
            .expect("tree-sitter markdown incremental parse should succeed");
    }

    pub(crate) fn block_seeds(&mut self, source: &Rope) -> Vec<BlockSeed> {
        self.preview_by_content_range.clear();
        let mut seeds = Vec::new();
        collect_block_seeds(
            self.tree.block_tree().root_node(),
            &self.tree,
            source,
            &mut self.preview_by_content_range,
            &mut seeds,
        );
        seeds
    }

    #[cfg(test)]
    pub(crate) fn preview_for_content_range(&self, range: &Range<usize>) -> Option<&PreviewBlock> {
        self.preview_by_content_range.get(&(range.start, range.end))
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

fn parse_rope(
    parser: &mut MarkdownParser,
    source: &Rope,
    old_tree: Option<&MarkdownTree>,
) -> Option<MarkdownTree> {
    let len = source.len_bytes();
    parser.parse_with(
        &mut |byte, _| {
            let clamped = cmp::min(byte, len);
            let (chunk, chunk_start, _, _) = source.chunk_at_byte(clamped);
            &chunk.as_bytes()[clamped - chunk_start..]
        },
        old_tree,
    )
}

fn collect_block_seeds(
    node: Node<'_>,
    tree: &MarkdownTree,
    source: &Rope,
    previews: &mut HashMap<(usize, usize), PreviewBlock>,
    seeds: &mut Vec<BlockSeed>,
) {
    if !node.is_named() {
        return;
    }

    match node.kind() {
        "document" | "section" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                collect_block_seeds(child, tree, source, previews, seeds);
            }
        }
        _ => {
            if let Some(seed) = block_seed_from_node(node, source) {
                if let Some(preview) = preview_from_node(node, tree, source) {
                    previews.insert((seed.content_range.start, seed.content_range.end), preview);
                }
                seeds.push(seed);
            }
        }
    }
}

fn block_seed_from_node(node: Node<'_>, source: &Rope) -> Option<BlockSeed> {
    let start_byte = node.start_byte();
    let content_end = content_end_for_node(source, node);
    let content_range = start_byte..content_end;
    let (kind, cursor_anchor_policy, can_code_edit) = match node.kind() {
        "paragraph" => {
            let text = source_text(source, start_byte..content_end);
            if is_math_block_text(&text) {
                (
                    BlockKind::MathBlock,
                    CursorAnchorPolicy::PreserveColumn,
                    false,
                )
            } else if is_footnote_definition_text(&text) {
                (
                    BlockKind::FootnoteDefinition,
                    CursorAnchorPolicy::Clamp,
                    false,
                )
            } else {
                (BlockKind::Paragraph, CursorAnchorPolicy::Clamp, false)
            }
        }
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
            (BlockKind::YamlFrontMatter, CursorAnchorPolicy::Clamp, false)
        }
        "link_reference_definition" => (
            BlockKind::FootnoteDefinition,
            CursorAnchorPolicy::Clamp,
            false,
        ),
        _ => recover_block_shape(source, start_byte, node.end_byte()).unwrap_or((
            BlockKind::Unknown,
            CursorAnchorPolicy::Clamp,
            false,
        )),
    };

    Some(BlockSeed {
        kind,
        content_range,
        cursor_anchor_policy,
        can_code_edit,
    })
}

fn preview_from_node(node: Node<'_>, tree: &MarkdownTree, source: &Rope) -> Option<PreviewBlock> {
    let preview = match node.kind() {
        "paragraph" => PreviewBlock::Paragraph {
            content: inline_segments_from_block(node, tree, source),
        },
        "atx_heading" => PreviewBlock::Heading {
            depth: atx_heading_depth(node),
            content: inline_segments_from_heading(node, tree, source),
        },
        "setext_heading" => PreviewBlock::Heading {
            depth: setext_heading_depth(node),
            content: inline_segments_from_setext_heading(node, tree, source),
        },
        "block_quote" => PreviewBlock::Blockquote {
            blocks: preview_nested_blocks(node, tree, source),
        },
        "list" => PreviewBlock::List {
            items: preview_list_items(node, tree, source),
        },
        "pipe_table" => preview_table(node, tree, source),
        "fenced_code_block" => PreviewBlock::CodeFence {
            language: fenced_code_language(node, source),
            text: fenced_code_text(node, source),
        },
        "indented_code_block" => PreviewBlock::CodeFence {
            language: None,
            text: indented_code_text(node, source),
        },
        "thematic_break" => PreviewBlock::ThematicBreak,
        "html_block" => PreviewBlock::Html {
            text: source_text(
                source,
                node.start_byte()..content_end_for_node(source, node),
            ),
        },
        "minus_metadata" | "plus_metadata" => PreviewBlock::Raw {
            text: source_text(
                source,
                node.start_byte()..content_end_for_node(source, node),
            ),
        },
        "link_reference_definition" => PreviewBlock::Unknown {
            text: source_text(
                source,
                node.start_byte()..content_end_for_node(source, node),
            ),
        },
        _ => PreviewBlock::Unknown {
            text: source_text(
                source,
                node.start_byte()..content_end_for_node(source, node),
            ),
        },
    };

    Some(preview)
}

fn preview_nested_blocks(node: Node<'_>, tree: &MarkdownTree, source: &Rope) -> Vec<PreviewBlock> {
    let mut blocks = Vec::new();
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_preview_child(child, tree, source, &mut blocks);
    }
    blocks
}

fn collect_preview_child(
    node: Node<'_>,
    tree: &MarkdownTree,
    source: &Rope,
    blocks: &mut Vec<PreviewBlock>,
) {
    if !node.is_named() {
        return;
    }

    match node.kind() {
        "section" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                collect_preview_child(child, tree, source, blocks);
            }
        }
        kind if is_structural_child_kind(kind) => {}
        _ => {
            if let Some(preview) = preview_from_node(node, tree, source) {
                blocks.push(preview);
            }
        }
    }
}

fn is_structural_child_kind(kind: &str) -> bool {
    matches!(
        kind,
        "block_continuation"
            | "block_quote_marker"
            | "list_marker_dot"
            | "list_marker_minus"
            | "list_marker_parenthesis"
            | "list_marker_plus"
            | "list_marker_star"
            | "task_list_marker_checked"
            | "task_list_marker_unchecked"
            | "setext_h1_underline"
            | "setext_h2_underline"
            | "pipe_table_delimiter_row"
            | "pipe_table_delimiter_cell"
            | "fenced_code_block_delimiter"
            | "info_string"
            | "language"
            | "code_fence_content"
    )
}

fn preview_list_items(node: Node<'_>, _tree: &MarkdownTree, source: &Rope) -> Vec<PreviewListItem> {
    let mut items = Vec::new();
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() != "list_item" {
            continue;
        }

        let mut blocks = preview_nested_blocks(child, _tree, source);
        if blocks.is_empty() {
            blocks.push(PreviewBlock::Paragraph {
                content: parse_inline_segments(&list_item_body_text(child, source)),
            });
        }

        items.push(PreviewListItem {
            marker: preview_list_marker(child, source),
            blocks,
        });
    }
    items
}

fn preview_list_marker(node: Node<'_>, source: &Rope) -> PreviewListMarker {
    let mut fallback = PreviewListMarker::Bullet;
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "task_list_marker_checked" => return PreviewListMarker::Task { checked: true },
            "task_list_marker_unchecked" => return PreviewListMarker::Task { checked: false },
            "list_marker_dot" | "list_marker_parenthesis" => {
                fallback = PreviewListMarker::Ordered(source_text(
                    source,
                    child.start_byte()..child.end_byte(),
                ));
            }
            "list_marker_minus" | "list_marker_plus" | "list_marker_star" => {
                fallback = PreviewListMarker::Bullet;
            }
            _ => {}
        }
    }

    fallback
}

fn list_item_body_text(node: Node<'_>, source: &Rope) -> String {
    let mut lines = source_text(
        source,
        node.start_byte()..content_end_for_node(source, node),
    )
    .lines()
    .map(str::to_string)
    .collect::<Vec<_>>();
    if lines.is_empty() {
        return String::new();
    }

    if let Some(first) = lines.first_mut() {
        let trimmed = first.trim_start();
        let without_marker = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
            .or_else(|| trimmed.strip_prefix("+ "))
            .or_else(|| strip_ordered_list_marker(trimmed))
            .unwrap_or(trimmed);
        let without_task = without_marker
            .strip_prefix("[x] ")
            .or_else(|| without_marker.strip_prefix("[X] "))
            .or_else(|| without_marker.strip_prefix("[ ] "))
            .unwrap_or(without_marker);
        *first = without_task.to_string();
    }

    lines.join("\n")
}

fn strip_ordered_list_marker(text: &str) -> Option<&str> {
    let digit_len = text
        .bytes()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    if digit_len == 0 {
        return None;
    }

    let marker = text.as_bytes().get(digit_len).copied();
    if !matches!(marker, Some(b'.' | b')')) {
        return None;
    }

    text.get(digit_len + 1..)
        .and_then(|rest| rest.strip_prefix(' '))
}

fn preview_table(node: Node<'_>, _tree: &MarkdownTree, source: &Rope) -> PreviewBlock {
    let mut header = Vec::new();
    let mut rows = Vec::new();
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "pipe_table_header" => header = preview_table_row(child, source),
            "pipe_table_row" => rows.push(preview_table_row(child, source)),
            _ => {}
        }
    }

    PreviewBlock::Table { header, rows }
}

fn preview_table_row(node: Node<'_>, source: &Rope) -> Vec<Vec<InlineSegment>> {
    let mut cells = Vec::new();
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() != "pipe_table_cell" {
            continue;
        }

        let mut segments = parse_inline_segments(
            source_text(source, child.start_byte()..child.end_byte())
                .trim()
                .trim_matches('|'),
        );
        if segments.is_empty() {
            let text = source_text(source, child.start_byte()..child.end_byte())
                .trim()
                .to_string();
            if !text.is_empty() {
                segments.push(InlineSegment {
                    text,
                    style: InlineStyle::default(),
                });
            }
        }
        cells.push(segments);
    }
    cells
}

fn inline_segments_from_block(
    node: Node<'_>,
    _tree: &MarkdownTree,
    source: &Rope,
) -> Vec<InlineSegment> {
    parse_inline_segments(&source_text(
        source,
        node.start_byte()..content_end_for_node(source, node),
    ))
}

fn inline_segments_from_heading(
    node: Node<'_>,
    _tree: &MarkdownTree,
    source: &Rope,
) -> Vec<InlineSegment> {
    node.child_by_field_name("heading_content")
        .map(|inline| {
            parse_inline_segments(&source_text(source, inline.start_byte()..inline.end_byte()))
        })
        .unwrap_or_default()
}

fn inline_segments_from_setext_heading(
    node: Node<'_>,
    _tree: &MarkdownTree,
    source: &Rope,
) -> Vec<InlineSegment> {
    node.child_by_field_name("heading_content")
        .map(|paragraph| {
            parse_inline_segments(&source_text(
                source,
                paragraph.start_byte()..content_end_for_node(source, paragraph),
            ))
        })
        .unwrap_or_default()
}

fn parse_inline_segments(text: &str) -> Vec<InlineSegment> {
    let mut segments = Vec::new();
    parse_inline_segments_into(text, &InlineStyle::default(), &mut segments);
    segments
}

fn parse_inline_segments_into(text: &str, style: &InlineStyle, segments: &mut Vec<InlineSegment>) {
    let mut offset = 0usize;
    while offset < text.len() {
        let rest = &text[offset..];

        if let Some(escaped) = rest.strip_prefix('\\') {
            if let Some(ch) = escaped.chars().next() {
                push_inline_text(segments, ch.to_string(), style);
                offset += 1 + ch.len_utf8();
                continue;
            }
        }

        if let Some((delimiter, advance)) = [("**", 2usize), ("__", 2usize), ("~~", 2usize)]
            .into_iter()
            .find(|(delimiter, _)| rest.starts_with(*delimiter))
        {
            if let Some(end) = text[offset + advance..].find(delimiter) {
                let mut nested = style.clone();
                match delimiter {
                    "**" | "__" => nested.strong = true,
                    "~~" => nested.strikethrough = true,
                    _ => {}
                }
                let inner_start = offset + advance;
                let inner_end = inner_start + end;
                parse_inline_segments_into(&text[inner_start..inner_end], &nested, segments);
                offset = inner_end + advance;
                continue;
            }
        }

        if let Some(end) = rest
            .strip_prefix('`')
            .and_then(|tail| tail.find('`').map(|end| (tail, end)))
        {
            let mut code = style.clone();
            code.code = true;
            push_inline_text(segments, end.0[..end.1].to_string(), &code);
            offset += end.1 + 2;
            continue;
        }

        if rest.starts_with('[')
            && let Some(close) = rest.find(']')
            && rest[close + 1..].starts_with('(')
            && let Some(end_paren) = rest[close + 2..].find(')')
        {
            let mut link = style.clone();
            link.link = true;
            parse_inline_segments_into(&rest[1..close], &link, segments);
            offset += close + 3 + end_paren;
            continue;
        }

        if rest.starts_with('<')
            && let Some(close) = rest.find('>')
            && close > 1
            && rest[1..close].contains([':', '@'])
        {
            let mut link = style.clone();
            link.link = true;
            push_inline_text(segments, rest[1..close].to_string(), &link);
            offset += close + 1;
            continue;
        }

        if let Some((delimiter, advance)) = [("*", 1usize), ("_", 1usize)]
            .into_iter()
            .find(|(delimiter, _)| rest.starts_with(*delimiter))
        {
            if let Some(end) = text[offset + advance..].find(delimiter) {
                let mut nested = style.clone();
                nested.emphasis = true;
                let inner_start = offset + advance;
                let inner_end = inner_start + end;
                parse_inline_segments_into(&text[inner_start..inner_end], &nested, segments);
                offset = inner_end + advance;
                continue;
            }
        }

        let next_special = rest
            .char_indices()
            .skip(1)
            .find(|(_, ch)| matches!(ch, '\\' | '*' | '_' | '~' | '`' | '[' | '<'))
            .map(|(idx, _)| idx)
            .unwrap_or(rest.len());
        push_inline_text(segments, rest[..next_special].to_string(), style);
        offset += next_special;
    }
}

fn push_inline_text(segments: &mut Vec<InlineSegment>, text: String, style: &InlineStyle) {
    if text.is_empty() {
        return;
    }

    if let Some(last) = segments.last_mut() {
        if last.style == *style {
            last.text.push_str(&text);
            return;
        }
    }

    segments.push(InlineSegment {
        text,
        style: style.clone(),
    });
}

fn find_child_by_kind<'tree>(node: Node<'tree>, kind: &str) -> Option<Node<'tree>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find(|child| child.kind() == kind)
}

fn fenced_code_text(node: Node<'_>, source: &Rope) -> String {
    if let Some(content) = find_child_by_kind(node, "code_fence_content") {
        let start = content.start_byte();
        let end = trimmed_content_end(source, start, content.end_byte());
        return source_text(source, start..end);
    }

    String::new()
}

fn indented_code_text(node: Node<'_>, source: &Rope) -> String {
    let text = source_text(
        source,
        node.start_byte()..content_end_for_node(source, node),
    );
    text.lines()
        .map(|line| {
            line.strip_prefix("    ")
                .or_else(|| line.strip_prefix('\t'))
                .unwrap_or(line)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn atx_heading_depth(node: Node<'_>) -> u8 {
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

fn setext_heading_depth(node: Node<'_>) -> u8 {
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

fn fenced_code_language(node: Node<'_>, source: &Rope) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() != "info_string" {
            continue;
        }

        let mut inner_cursor = child.walk();
        for inner in child.named_children(&mut inner_cursor) {
            if inner.kind() == "language" {
                let text = source_text(source, inner.start_byte()..inner.end_byte());
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }

        let text = source_text(source, child.start_byte()..child.end_byte());
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
    let text = source_text(source, start..end);
    if is_math_block_text(&text) {
        return Some((
            BlockKind::MathBlock,
            CursorAnchorPolicy::PreserveColumn,
            false,
        ));
    }
    if is_footnote_definition_text(&text) {
        return Some((
            BlockKind::FootnoteDefinition,
            CursorAnchorPolicy::Clamp,
            false,
        ));
    }
    if let Some(depth) = atx_heading_depth_from_text(&text) {
        return Some((
            BlockKind::Heading { depth },
            CursorAnchorPolicy::Clamp,
            false,
        ));
    }

    if looks_like_list_block(&text) {
        return Some((BlockKind::List, CursorAnchorPolicy::Clamp, false));
    }

    if looks_like_blockquote_block(&text) {
        return Some((BlockKind::Blockquote, CursorAnchorPolicy::Clamp, false));
    }

    None
}

fn is_math_block_text(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.len() >= 4 && trimmed.starts_with("$$") && trimmed.ends_with("$$")
}

fn is_footnote_definition_text(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed.starts_with("[^") && trimmed.contains("]:")
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

    let digit_len = line
        .bytes()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
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

fn content_end_for_node(source: &Rope, node: Node<'_>) -> usize {
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

fn source_text(source: &Rope, range: Range<usize>) -> String {
    if range.start >= range.end {
        return String::new();
    }

    source
        .get_byte_slice(range)
        .expect("document byte range should align to UTF-8 boundaries")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_block_kinds_from_tree_sitter_markdown() {
        let source = Rope::from_str(
            "# Heading\n\n- item\n\n> quote\n\n| a | b |\n| - | - |\n| 1 | 2 |\n\n```rust\nfn main() {}\n```\n",
        );
        let mut syntax = SyntaxState::from_source(&source);
        let seeds = syntax.block_seeds(&source);

        assert!(
            seeds
                .iter()
                .any(|seed| matches!(seed.kind, BlockKind::Heading { depth: 1 }))
        );
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
        let source = Rope::from_str("a\n浣犲ソ\n");
        let edit = input_edit_for_splice(&source, 2..5, "xyz");

        assert_eq!(edit.start_position, Point { row: 1, column: 0 });
        assert_eq!(edit.old_end_position, Point { row: 1, column: 3 });
        assert_eq!(edit.new_end_position, Point { row: 1, column: 3 });
    }

    #[test]
    fn builds_inline_preview_segments_from_incremental_tree() {
        let source = Rope::from_str("Alpha *beta* **gamma** `delta` [link](https://example.com)");
        let mut syntax = SyntaxState::from_source(&source);
        let seeds = syntax.block_seeds(&source);
        let preview = syntax
            .preview_for_content_range(&seeds[0].content_range)
            .expect("preview");

        let PreviewBlock::Paragraph { content } = preview else {
            panic!("expected paragraph preview");
        };

        assert_eq!(content[0].text, "Alpha ");
        assert!(content[1].style.emphasis);
        assert_eq!(content[1].text, "beta");
        assert!(content[3].style.strong);
        assert_eq!(content[3].text, "gamma");
        assert!(content[5].style.code);
        assert_eq!(content[5].text, "delta");
        assert!(content[7].style.link);
        assert_eq!(content[7].text, "link");
    }

    #[test]
    fn builds_task_list_preview_from_tree_sitter_nodes() {
        let source = Rope::from_str("- [x] done\n- [ ] todo\n");
        let mut syntax = SyntaxState::from_source(&source);
        let seeds = syntax.block_seeds(&source);
        let preview = syntax
            .preview_for_content_range(&seeds[0].content_range)
            .expect("preview");

        let PreviewBlock::List { items } = preview else {
            panic!("expected list preview");
        };

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].marker, PreviewListMarker::Task { checked: true });
        assert_eq!(items[1].marker, PreviewListMarker::Task { checked: false });
    }

    #[test]
    fn builds_table_preview_with_header_and_rows() {
        let source = Rope::from_str("| a | b |\n| - | - |\n| 1 | `2` |\n");
        let mut syntax = SyntaxState::from_source(&source);
        let seeds = syntax.block_seeds(&source);
        let preview = syntax
            .preview_for_content_range(&seeds[0].content_range)
            .expect("preview");

        let PreviewBlock::Table { header, rows } = preview else {
            panic!("expected table preview");
        };

        assert_eq!(header.len(), 2);
        assert_eq!(header[0][0].text, "a");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0][0][0].text, "1");
        assert_eq!(rows[0][1][0].text, "2");
        assert!(rows[0][1][0].style.code);
    }
}
