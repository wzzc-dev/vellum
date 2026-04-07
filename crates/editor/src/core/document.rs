use std::{cmp, ops::Range};

use markdown::{ParseOptions, mdast::Node, to_mdast};
use ropey::Rope;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockKind {
    Raw,
    Paragraph,
    Heading { depth: u8 },
    Blockquote,
    List,
    Table,
    CodeFence { language: Option<String> },
    ThematicBreak,
    Html,
    Footnote,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorAnchorPolicy {
    Clamp,
    PreserveColumn,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockProjection {
    pub id: u64,
    pub kind: BlockKind,
    pub byte_range: Range<usize>,
    pub content_range: Range<usize>,
    pub cursor_anchor_policy: CursorAnchorPolicy,
    pub can_code_edit: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionState {
    pub anchor: usize,
    pub head: usize,
    pub preferred_column: Option<usize>,
}

impl SelectionState {
    pub fn collapsed(offset: usize) -> Self {
        Self {
            anchor: offset,
            head: offset,
            preferred_column: None,
        }
    }

    pub fn range(&self) -> Range<usize> {
        cmp::min(self.anchor, self.head)..cmp::max(self.anchor, self.head)
    }

    pub fn cursor(&self) -> usize {
        self.head
    }

    pub fn is_collapsed(&self) -> bool {
        self.anchor == self.head
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Transaction {
    Replace {
        range: Range<usize>,
        replacement: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedTransaction {
    pub before_range: Range<usize>,
    pub before_text: String,
    pub after_range: Range<usize>,
    pub after_text: String,
}

#[derive(Debug, Clone)]
pub struct DocumentBuffer {
    source: Rope,
    blocks: Vec<BlockProjection>,
    parse_version: u64,
    next_block_id: u64,
}

pub type BlockSpan = BlockProjection;
pub type DocumentState = DocumentBuffer;

impl DocumentBuffer {
    pub fn new_empty() -> Self {
        let mut this = Self {
            source: Rope::new(),
            blocks: Vec::new(),
            parse_version: 0,
            next_block_id: 1,
        };
        this.reparse(None, &[]);
        this
    }

    pub fn from_text(text: impl AsRef<str>) -> Self {
        let mut this = Self {
            source: Rope::from_str(text.as_ref()),
            blocks: Vec::new(),
            parse_version: 0,
            next_block_id: 1,
        };
        this.reparse(None, &[]);
        this
    }

    pub fn text(&self) -> String {
        self.source.to_string()
    }

    pub fn len(&self) -> usize {
        self.source.len_bytes()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn blocks(&self) -> &[BlockProjection] {
        &self.blocks
    }

    pub fn parse_version(&self) -> u64 {
        self.parse_version
    }

    pub fn block_index_by_id(&self, block_id: u64) -> Option<usize> {
        self.blocks.iter().position(|block| block.id == block_id)
    }

    pub fn block_by_id(&self, block_id: u64) -> Option<&BlockProjection> {
        self.blocks.iter().find(|block| block.id == block_id)
    }

    pub fn block_index_at_offset(&self, offset: usize) -> usize {
        if self.blocks.is_empty() {
            return 0;
        }

        let clipped = cmp::min(offset, self.len());
        if let Some(index) = self.blocks.iter().rposition(|block| {
            clipped == block.byte_range.start
                && (block.content_range.is_empty() || block.kind != BlockKind::Raw)
        }) {
            return index;
        }

        self.blocks
            .iter()
            .position(|block| clipped >= block.byte_range.start && clipped <= block.byte_range.end)
            .unwrap_or_else(|| self.blocks.len().saturating_sub(1))
    }

    pub fn block_text(&self, block: &BlockProjection) -> String {
        self.text_for_range(block.content_range.clone())
    }

    pub fn block_span_text(&self, block: &BlockProjection) -> String {
        self.text_for_range(block.byte_range.clone())
    }

    pub fn block_trailing_text(&self, block: &BlockProjection) -> String {
        self.text_for_range(block.content_range.end..block.byte_range.end)
    }

    pub fn text_for_range(&self, range: Range<usize>) -> String {
        if range.start >= range.end {
            return String::new();
        }

        self.source
            .get_byte_slice(range)
            .expect("document byte range should align to UTF-8 boundaries")
            .to_string()
    }

    pub fn apply_transaction(&mut self, transaction: Transaction) -> AppliedTransaction {
        match transaction {
            Transaction::Replace { range, replacement } => {
                let before_text = self.text_for_range(range.clone());
                self.replace_range(range.clone(), &replacement);
                AppliedTransaction {
                    before_range: range.clone(),
                    before_text,
                    after_range: range.start..range.start + replacement.len(),
                    after_text: replacement,
                }
            }
        }
    }

    pub fn replace_range(&mut self, range: Range<usize>, replacement: &str) {
        let previous_source = self.text();
        let previous_blocks = self.blocks.clone();
        let start = cmp::min(range.start, self.len());
        let end = cmp::min(range.end, self.len());
        let start_char = self
            .source
            .try_byte_to_char(start)
            .expect("document byte range should align to UTF-8 boundaries");
        let end_char = self
            .source
            .try_byte_to_char(end)
            .expect("document byte range should align to UTF-8 boundaries");

        self.source.remove(start_char..end_char);
        self.source.insert(start_char, replacement);
        self.reparse(Some(previous_source.as_str()), &previous_blocks);
    }

    fn reparse(&mut self, previous_source: Option<&str>, previous_blocks: &[BlockProjection]) {
        self.parse_version = self.parse_version.wrapping_add(1);
        let source = self.text();
        let mut parsed = parse_blocks(&source);
        assign_block_ids(
            &mut parsed,
            &source,
            previous_source,
            previous_blocks,
            &mut self.next_block_id,
        );
        self.blocks = parsed;
        if self.blocks.is_empty() {
            self.blocks.push(BlockProjection {
                id: take_next_block_id(&mut self.next_block_id),
                kind: BlockKind::Raw,
                byte_range: 0..0,
                content_range: 0..0,
                cursor_anchor_policy: CursorAnchorPolicy::Clamp,
                can_code_edit: false,
            });
        }
    }
}

fn parse_blocks(source: &str) -> Vec<BlockProjection> {
    if source.is_empty() {
        return vec![BlockProjection {
            id: 0,
            kind: BlockKind::Raw,
            byte_range: 0..0,
            content_range: 0..0,
            cursor_anchor_policy: CursorAnchorPolicy::Clamp,
            can_code_edit: false,
        }];
    }

    let tree = to_mdast(source, &ParseOptions::gfm()).ok();
    let Some(Node::Root(root)) = tree else {
        return vec![BlockProjection {
            id: 0,
            kind: BlockKind::Raw,
            byte_range: 0..source.len(),
            content_range: 0..source.len(),
            cursor_anchor_policy: CursorAnchorPolicy::Clamp,
            can_code_edit: false,
        }];
    };

    if root.children.is_empty() {
        return vec![BlockProjection {
            id: 0,
            kind: BlockKind::Raw,
            byte_range: 0..source.len(),
            content_range: 0..source.len(),
            cursor_anchor_policy: CursorAnchorPolicy::Clamp,
            can_code_edit: false,
        }];
    }

    let mut blocks = Vec::new();
    let mut cursor = 0usize;

    for (index, node) in root.children.iter().enumerate() {
        let Some(position) = node.position() else {
            continue;
        };

        let start = cmp::min(position.start.offset, source.len());
        let content_end = cmp::min(position.end.offset, source.len());
        let end = root
            .children
            .get(index + 1)
            .and_then(Node::position)
            .map(|pos| cmp::min(pos.start.offset, source.len()))
            .unwrap_or(content_end);
        let span_end = cmp::max(content_end, end);

        if start > cursor {
            blocks.push(BlockProjection {
                id: 0,
                kind: BlockKind::Raw,
                byte_range: cursor..start,
                content_range: cursor..start,
                cursor_anchor_policy: CursorAnchorPolicy::Clamp,
                can_code_edit: false,
            });
        }

        blocks.push(BlockProjection {
            id: 0,
            kind: block_kind(node),
            byte_range: start..span_end,
            content_range: start..content_end,
            cursor_anchor_policy: cursor_policy(node),
            can_code_edit: matches!(node, Node::Code(_)),
        });
        cursor = span_end;
    }

    if cursor < source.len() {
        blocks.push(BlockProjection {
            id: 0,
            kind: BlockKind::Raw,
            byte_range: cursor..source.len(),
            content_range: cursor..source.len(),
            cursor_anchor_policy: CursorAnchorPolicy::Clamp,
            can_code_edit: false,
        });
    }

    if blocks.is_empty() {
        blocks.push(BlockProjection {
            id: 0,
            kind: BlockKind::Raw,
            byte_range: 0..source.len(),
            content_range: 0..source.len(),
            cursor_anchor_policy: CursorAnchorPolicy::Clamp,
            can_code_edit: false,
        });
    }

    materialize_trailing_empty_block(&mut blocks, source);

    blocks
}

fn materialize_trailing_empty_block(blocks: &mut Vec<BlockProjection>, source: &str) {
    let Some(trailing) = blocks.last().cloned() else {
        return;
    };
    if trailing.kind != BlockKind::Raw {
        return;
    }
    if blocks.len() < 2 || blocks[blocks.len() - 2].kind == BlockKind::Raw {
        return;
    }

    let previous_text = source_text(source, blocks[blocks.len() - 2].content_range.clone());
    let trailing_text = source_text(source, trailing.byte_range.clone());
    if !is_trailing_block_separator(previous_text, trailing_text) {
        return;
    }

    let separator_end = trailing.byte_range.end;
    blocks.pop();
    if let Some(previous) = blocks.last_mut() {
        previous.byte_range.end = separator_end;
    }
    blocks.push(BlockProjection {
        id: 0,
        kind: BlockKind::Raw,
        byte_range: source.len()..source.len(),
        content_range: source.len()..source.len(),
        cursor_anchor_policy: CursorAnchorPolicy::Clamp,
        can_code_edit: false,
    });
}

fn assign_block_ids(
    blocks: &mut [BlockProjection],
    source: &str,
    previous_source: Option<&str>,
    previous_blocks: &[BlockProjection],
    next_block_id: &mut u64,
) {
    if let Some(previous_source) = previous_source.filter(|_| !previous_blocks.is_empty()) {
        let mut previous_prefix = 0usize;
        let mut next_prefix = 0usize;
        while previous_prefix < previous_blocks.len()
            && next_prefix < blocks.len()
            && same_block_signature(
                &previous_blocks[previous_prefix],
                previous_source,
                &blocks[next_prefix],
                source,
            )
        {
            blocks[next_prefix].id = previous_blocks[previous_prefix].id;
            previous_prefix += 1;
            next_prefix += 1;
        }

        let mut previous_suffix = previous_blocks.len();
        let mut next_suffix = blocks.len();
        while previous_suffix > previous_prefix
            && next_suffix > next_prefix
            && same_block_signature(
                &previous_blocks[previous_suffix - 1],
                previous_source,
                &blocks[next_suffix - 1],
                source,
            )
        {
            previous_suffix -= 1;
            next_suffix -= 1;
            blocks[next_suffix].id = previous_blocks[previous_suffix].id;
        }

        for (block, previous_block) in blocks[next_prefix..next_suffix]
            .iter_mut()
            .zip(previous_blocks[previous_prefix..previous_suffix].iter())
        {
            block.id = previous_block.id;
        }
    }

    for block in blocks.iter_mut().filter(|block| block.id == 0) {
        block.id = take_next_block_id(next_block_id);
    }
}

fn same_block_signature(
    previous_block: &BlockProjection,
    previous_source: &str,
    block: &BlockProjection,
    source: &str,
) -> bool {
    previous_block.kind == block.kind
        && previous_block.cursor_anchor_policy == block.cursor_anchor_policy
        && previous_block.can_code_edit == block.can_code_edit
        && source_text(previous_source, previous_block.byte_range.clone())
            == source_text(source, block.byte_range.clone())
}

fn source_text(source: &str, range: Range<usize>) -> &str {
    source.get(range).unwrap_or_default()
}

fn is_trailing_block_separator(previous_text: &str, trailing_text: &str) -> bool {
    if !trailing_text
        .trim_matches([' ', '\t', '\r', '\n'])
        .is_empty()
    {
        return false;
    }

    trailing_newline_count(previous_text) + trailing_newline_count(trailing_text) >= 2
}

fn trailing_newline_count(text: &str) -> usize {
    text.bytes()
        .rev()
        .take_while(|byte| matches!(byte, b'\n' | b'\r'))
        .filter(|byte| *byte == b'\n')
        .count()
}

fn take_next_block_id(next_block_id: &mut u64) -> u64 {
    let id = *next_block_id;
    *next_block_id = next_block_id.wrapping_add(1);
    id
}

fn block_kind(node: &Node) -> BlockKind {
    match node {
        Node::Paragraph(_) => BlockKind::Paragraph,
        Node::Heading(heading) => BlockKind::Heading {
            depth: heading.depth,
        },
        Node::Blockquote(_) => BlockKind::Blockquote,
        Node::List(_) => BlockKind::List,
        Node::Table(_) => BlockKind::Table,
        Node::Code(code) => BlockKind::CodeFence {
            language: code.lang.clone(),
        },
        Node::ThematicBreak(_) => BlockKind::ThematicBreak,
        Node::Html(_) => BlockKind::Html,
        Node::FootnoteDefinition(_) => BlockKind::Footnote,
        Node::Yaml(_) | Node::Toml(_) => BlockKind::Raw,
        _ => BlockKind::Unknown,
    }
}

fn cursor_policy(node: &Node) -> CursorAnchorPolicy {
    if matches!(node, Node::Code(_)) {
        CursorAnchorPolicy::PreserveColumn
    } else {
        CursorAnchorPolicy::Clamp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_empty_document_into_single_raw_block() {
        let doc = DocumentBuffer::new_empty();
        assert_eq!(doc.blocks.len(), 1);
        assert_eq!(doc.blocks[0].kind, BlockKind::Raw);
        assert_eq!(doc.blocks[0].byte_range, 0..0);
    }

    #[test]
    fn parses_core_gfm_blocks() {
        let text = concat!(
            "# Heading\n\n",
            "- [ ] task\n- item\n\n",
            "> quote\n\n",
            "| a | b |\n| - | - |\n| 1 | 2 |\n\n",
            "```rust\nfn main() {}\n```\n"
        );
        let doc = DocumentBuffer::from_text(text);
        assert!(
            doc.blocks
                .iter()
                .any(|block| matches!(block.kind, BlockKind::Heading { .. }))
        );
        assert!(doc.blocks.iter().any(|block| block.kind == BlockKind::List));
        assert!(
            doc.blocks
                .iter()
                .any(|block| block.kind == BlockKind::Blockquote)
        );
        assert!(
            doc.blocks
                .iter()
                .any(|block| block.kind == BlockKind::Table)
        );
        assert!(
            doc.blocks
                .iter()
                .any(|block| matches!(block.kind, BlockKind::CodeFence { .. }))
        );
    }

    #[test]
    fn preserves_leading_raw_content_before_first_ast_block() {
        let text = "\n\n# Title\n";
        let doc = DocumentBuffer::from_text(text);
        assert_eq!(doc.blocks[0].kind, BlockKind::Raw);
        assert_eq!(doc.block_text(&doc.blocks[0]), "\n\n");
        assert!(matches!(doc.blocks[1].kind, BlockKind::Heading { .. }));
    }

    #[test]
    fn excludes_inter_block_separator_from_block_text() {
        let doc = DocumentBuffer::from_text("First\n\nSecond\n");
        assert_eq!(doc.block_text(&doc.blocks[0]), "First");
        assert_eq!(doc.block_trailing_text(&doc.blocks[0]), "\n\n");
    }

    #[test]
    fn reparses_after_splice_and_tracks_block_merges() {
        let mut doc = DocumentBuffer::from_text("# Title\n\nParagraph\n");
        let first_parse = doc.parse_version();
        let paragraph = doc
            .blocks
            .iter()
            .find(|block| block.kind == BlockKind::Paragraph)
            .cloned()
            .expect("paragraph block");
        doc.replace_range(paragraph.byte_range, "Paragraph\n\n## Child\n");
        assert!(doc.parse_version() > first_parse);
        assert!(
            doc.blocks
                .iter()
                .any(|block| matches!(block.kind, BlockKind::Heading { depth: 2 }))
        );
    }

    #[test]
    fn preserves_block_ids_for_in_place_edits_and_unchanged_neighbors() {
        let mut doc = DocumentBuffer::from_text("First\n\nSecond\n");
        let first = doc.blocks[0].clone();
        let second = doc.blocks[1].clone();

        doc.replace_range(first.byte_range.clone(), "Changed\n\n");

        assert_eq!(doc.blocks[0].id, first.id);
        assert_eq!(doc.blocks[1].id, second.id);
        assert_eq!(doc.block_text(&doc.blocks[0]), "Changed");
        assert_eq!(doc.block_text(&doc.blocks[1]), "Second");
    }

    #[test]
    fn preserves_block_id_when_block_kind_changes_in_place() {
        let mut doc = DocumentBuffer::from_text("Title\n");
        let original = doc.blocks[0].clone();

        doc.replace_range(original.byte_range.clone(), "# Title\n");

        assert_eq!(doc.blocks[0].id, original.id);
        assert!(matches!(
            doc.blocks[0].kind,
            BlockKind::Heading { depth: 1 }
        ));
    }

    #[test]
    fn maps_offsets_back_to_current_blocks() {
        let doc = DocumentBuffer::from_text("# A\n\nParagraph\n\n```rs\nfn main() {}\n```\n");
        let paragraph = doc
            .blocks
            .iter()
            .find(|block| block.kind == BlockKind::Paragraph)
            .expect("paragraph");
        let code = doc
            .blocks
            .iter()
            .find(|block| matches!(block.kind, BlockKind::CodeFence { .. }))
            .expect("code");
        let paragraph_ix = doc.block_index_at_offset(paragraph.byte_range.start + 1);
        let code_ix = doc.block_index_at_offset(code.byte_range.start + 2);
        assert_eq!(doc.blocks[paragraph_ix].kind, BlockKind::Paragraph);
        assert!(matches!(
            doc.blocks[code_ix].kind,
            BlockKind::CodeFence { .. }
        ));
    }

    #[test]
    fn applies_transactions_without_rebuilding_whole_document() {
        let mut doc = DocumentBuffer::from_text("# 标题\n\n段落\n");
        let paragraph = doc
            .blocks
            .iter()
            .find(|block| block.kind == BlockKind::Paragraph)
            .cloned()
            .expect("paragraph");

        let applied = doc.apply_transaction(Transaction::Replace {
            range: paragraph.byte_range,
            replacement: "更新后的段落\n".to_string(),
        });

        assert_eq!(applied.before_text, "段落");
        assert_eq!(applied.after_text, "更新后的段落\n");
        let paragraph = doc
            .blocks
            .iter()
            .find(|block| block.kind == BlockKind::Paragraph)
            .expect("paragraph after replace");
        assert_eq!(doc.block_text(paragraph), "更新后的段落");
        assert!(doc.text().contains("# 标题"));
    }

    #[test]
    fn tracks_collapsed_selection_ranges() {
        let selection = SelectionState::collapsed(5);
        assert!(selection.is_collapsed());
        assert_eq!(selection.range(), 5..5);
        assert_eq!(selection.cursor(), 5);
    }

    #[test]
    fn materializes_editable_trailing_empty_block_after_separator() {
        let doc = DocumentBuffer::from_text("First\n\n");

        assert_eq!(doc.blocks.len(), 2);
        assert_eq!(doc.blocks[0].kind, BlockKind::Paragraph);
        assert_eq!(doc.block_trailing_text(&doc.blocks[0]), "\n\n");
        assert_eq!(doc.blocks[1].kind, BlockKind::Raw);
        assert_eq!(doc.block_text(&doc.blocks[1]), "");
        assert_eq!(doc.blocks[1].byte_range, 7..7);
    }
}
