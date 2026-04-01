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
}

pub type BlockSpan = BlockProjection;
pub type DocumentState = DocumentBuffer;

impl DocumentBuffer {
    pub fn new_empty() -> Self {
        let mut this = Self {
            source: Rope::new(),
            blocks: Vec::new(),
            parse_version: 0,
        };
        this.reparse();
        this
    }

    pub fn from_text(text: impl AsRef<str>) -> Self {
        let mut this = Self {
            source: Rope::from_str(text.as_ref()),
            blocks: Vec::new(),
            parse_version: 0,
        };
        this.reparse();
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
        self.reparse();
    }

    fn reparse(&mut self) {
        self.parse_version = self.parse_version.wrapping_add(1);
        self.blocks = parse_blocks(&self.text(), self.parse_version);
        if self.blocks.is_empty() {
            self.blocks.push(BlockProjection {
                id: make_block_id(self.parse_version, 0),
                kind: BlockKind::Raw,
                byte_range: 0..0,
                content_range: 0..0,
                cursor_anchor_policy: CursorAnchorPolicy::Clamp,
                can_code_edit: false,
            });
        }
    }
}

fn make_block_id(parse_version: u64, index: usize) -> u64 {
    (parse_version << 32) | index as u64
}

fn parse_blocks(source: &str, parse_version: u64) -> Vec<BlockProjection> {
    if source.is_empty() {
        return vec![BlockProjection {
            id: make_block_id(parse_version, 0),
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
            id: make_block_id(parse_version, 0),
            kind: BlockKind::Raw,
            byte_range: 0..source.len(),
            content_range: 0..source.len(),
            cursor_anchor_policy: CursorAnchorPolicy::Clamp,
            can_code_edit: false,
        }];
    };

    if root.children.is_empty() {
        return vec![BlockProjection {
            id: make_block_id(parse_version, 0),
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
                id: make_block_id(parse_version, blocks.len()),
                kind: BlockKind::Raw,
                byte_range: cursor..start,
                content_range: cursor..start,
                cursor_anchor_policy: CursorAnchorPolicy::Clamp,
                can_code_edit: false,
            });
        }

        blocks.push(BlockProjection {
            id: make_block_id(parse_version, blocks.len()),
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
            id: make_block_id(parse_version, blocks.len()),
            kind: BlockKind::Raw,
            byte_range: cursor..source.len(),
            content_range: cursor..source.len(),
            cursor_anchor_policy: CursorAnchorPolicy::Clamp,
            can_code_edit: false,
        });
    }

    if blocks.is_empty() {
        blocks.push(BlockProjection {
            id: make_block_id(parse_version, 0),
            kind: BlockKind::Raw,
            byte_range: 0..source.len(),
            content_range: 0..source.len(),
            cursor_anchor_policy: CursorAnchorPolicy::Clamp,
            can_code_edit: false,
        });
    }

    blocks
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
}
