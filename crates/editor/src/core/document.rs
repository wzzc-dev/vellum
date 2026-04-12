use std::{
    cmp,
    collections::hash_map::DefaultHasher,
    fmt,
    hash::{Hash, Hasher},
    ops::Range,
};

use ropey::Rope;

use super::{
    display_map::{DisplayMap, HiddenSyntaxPolicy},
    syntax::{
        SyntaxState, input_edit_for_splice, looks_like_blockquote_block, looks_like_list_block,
    },
};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionAffinity {
    Upstream,
    Downstream,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionModel {
    pub anchor_byte: usize,
    pub head_byte: usize,
    pub preferred_column: Option<usize>,
    pub affinity: SelectionAffinity,
}

impl SelectionModel {
    pub fn collapsed(offset: usize) -> Self {
        Self {
            anchor_byte: offset,
            head_byte: offset,
            preferred_column: None,
            affinity: SelectionAffinity::Downstream,
        }
    }

    pub fn range(&self) -> Range<usize> {
        cmp::min(self.anchor_byte, self.head_byte)..cmp::max(self.anchor_byte, self.head_byte)
    }

    pub fn cursor(&self) -> usize {
        self.head_byte
    }

    pub fn is_collapsed(&self) -> bool {
        self.anchor_byte == self.head_byte
    }
}

pub type SelectionState = SelectionModel;

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

pub struct DocumentBuffer {
    source: Rope,
    syntax: SyntaxState,
    blocks: Vec<BlockProjection>,
    parse_version: u64,
    next_block_id: u64,
}

pub type BlockSpan = BlockProjection;
pub type DocumentState = DocumentBuffer;

#[derive(Debug, Clone, PartialEq, Eq)]
struct BlockSignature {
    id: u64,
    kind: BlockKind,
    byte_len: usize,
    span_hash: u64,
    cursor_anchor_policy: CursorAnchorPolicy,
    can_code_edit: bool,
}

impl Clone for DocumentBuffer {
    fn clone(&self) -> Self {
        Self {
            source: self.source.clone(),
            syntax: SyntaxState::from_source(&self.source),
            blocks: self.blocks.clone(),
            parse_version: self.parse_version,
            next_block_id: self.next_block_id,
        }
    }
}

impl fmt::Debug for DocumentBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DocumentBuffer")
            .field("source_len_bytes", &self.source.len_bytes())
            .field("blocks", &self.blocks)
            .field("parse_version", &self.parse_version)
            .field("next_block_id", &self.next_block_id)
            .finish()
    }
}

impl DocumentBuffer {
    pub fn new_empty() -> Self {
        Self::from_rope(Rope::new())
    }

    pub fn from_text(text: impl AsRef<str>) -> Self {
        Self::from_rope(Rope::from_str(text.as_ref()))
    }

    fn from_rope(source: Rope) -> Self {
        let syntax = SyntaxState::from_source(&source);
        let mut this = Self {
            source,
            syntax,
            blocks: Vec::new(),
            parse_version: 0,
            next_block_id: 1,
        };
        this.rebuild_blocks(&[]);
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

    pub fn display_map(&self, selection: Option<&SelectionModel>) -> DisplayMap {
        DisplayMap::from_document(self, selection, HiddenSyntaxPolicy::SelectionAware)
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
        let start = cmp::min(range.start, self.len());
        let end = cmp::min(range.end, self.len());
        let previous_signatures = self.block_signatures();
        let edit = input_edit_for_splice(&self.source, start..end, replacement);

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
        self.syntax.reparse(&self.source, edit);
        self.rebuild_blocks(&previous_signatures);
    }

    fn rebuild_blocks(&mut self, previous_signatures: &[BlockSignature]) {
        self.parse_version = self.parse_version.wrapping_add(1);
        let mut parsed = parse_blocks(&self.source, &mut self.syntax);
        if !projection_invariants_hold(&parsed, &self.source) {
            self.syntax = SyntaxState::from_source(&self.source);
            parsed = parse_blocks(&self.source, &mut self.syntax);
            debug_assert!(
                projection_invariants_hold(&parsed, &self.source),
                "full tree-sitter reparse should restore projection invariants"
            );
        }
        assign_block_ids(
            &mut parsed,
            previous_signatures,
            &self.source,
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

    fn block_signatures(&self) -> Vec<BlockSignature> {
        self.blocks
            .iter()
            .map(|block| block_signature(&self.source, block))
            .collect()
    }
}

fn parse_blocks(source: &Rope, syntax: &mut SyntaxState) -> Vec<BlockProjection> {
    if source.len_bytes() == 0 {
        return vec![BlockProjection {
            id: 0,
            kind: BlockKind::Raw,
            byte_range: 0..0,
            content_range: 0..0,
            cursor_anchor_policy: CursorAnchorPolicy::Clamp,
            can_code_edit: false,
        }];
    }

    let seeds = syntax.block_seeds(source);
    if seeds.is_empty() {
        return vec![BlockProjection {
            id: 0,
            kind: BlockKind::Raw,
            byte_range: 0..source.len_bytes(),
            content_range: 0..source.len_bytes(),
            cursor_anchor_policy: CursorAnchorPolicy::Clamp,
            can_code_edit: false,
        }];
    }

    let mut blocks = Vec::new();
    let mut cursor = 0usize;

    for (index, seed) in seeds.iter().enumerate() {
        let start = cmp::min(seed.content_range.start, source.len_bytes());
        let content_end = cmp::min(seed.content_range.end, source.len_bytes());
        let end = seeds
            .get(index + 1)
            .map(|next| cmp::min(next.content_range.start, source.len_bytes()))
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
            kind: seed.kind.clone(),
            byte_range: start..span_end,
            content_range: start..content_end,
            cursor_anchor_policy: seed.cursor_anchor_policy,
            can_code_edit: seed.can_code_edit,
        });
        cursor = span_end;
    }

    if cursor < source.len_bytes() {
        blocks.push(BlockProjection {
            id: 0,
            kind: BlockKind::Raw,
            byte_range: cursor..source.len_bytes(),
            content_range: cursor..source.len_bytes(),
            cursor_anchor_policy: CursorAnchorPolicy::Clamp,
            can_code_edit: false,
        });
    }

    if blocks.is_empty() {
        blocks.push(BlockProjection {
            id: 0,
            kind: BlockKind::Raw,
            byte_range: 0..source.len_bytes(),
            content_range: 0..source.len_bytes(),
            cursor_anchor_policy: CursorAnchorPolicy::Clamp,
            can_code_edit: false,
        });
    }

    merge_structured_continuations(&mut blocks, source);
    materialize_inter_block_empty_blocks(&mut blocks, source);
    materialize_trailing_empty_block(&mut blocks, source);

    blocks
}

fn merge_structured_continuations(blocks: &mut Vec<BlockProjection>, source: &Rope) {
    if blocks.len() < 2 {
        return;
    }

    let mut merged = Vec::with_capacity(blocks.len());
    let mut index = 0usize;
    while index < blocks.len() {
        let mut current = blocks[index].clone();
        index += 1;

        while index < blocks.len()
            && can_merge_structured_continuation(source, &current, &blocks[index])
        {
            let next = &blocks[index];
            current.byte_range.end = next.byte_range.end;
            current.content_range.end = next.content_range.end;
            index += 1;
        }

        merged.push(current);
    }

    *blocks = merged;
}

fn can_merge_structured_continuation(
    source: &Rope,
    current: &BlockProjection,
    next: &BlockProjection,
) -> bool {
    let separator = source_text(source, current.content_range.end..next.byte_range.start);
    if !is_single_line_whitespace_separator(&separator) {
        return false;
    }

    let next_text = source_text(source, next.content_range.clone());
    match current.kind {
        BlockKind::List => {
            matches!(
                next.kind,
                BlockKind::List | BlockKind::Unknown | BlockKind::Raw
            ) && looks_like_list_block(&next_text)
        }
        BlockKind::Blockquote => {
            matches!(
                next.kind,
                BlockKind::Blockquote | BlockKind::Unknown | BlockKind::Raw
            ) && looks_like_blockquote_block(&next_text)
        }
        _ => false,
    }
}

fn is_single_line_whitespace_separator(text: &str) -> bool {
    text.trim_matches([' ', '\t', '\r', '\n']).is_empty() && trailing_newline_count(text) <= 1
}

fn materialize_inter_block_empty_blocks(blocks: &mut Vec<BlockProjection>, source: &Rope) {
    if blocks.len() < 2 {
        return;
    }

    let mut materialized = Vec::with_capacity(blocks.len());
    for (index, block) in blocks.iter().cloned().enumerate() {
        materialized.push(block.clone());

        let Some(next) = blocks.get(index + 1) else {
            continue;
        };
        if block.kind == BlockKind::Raw || next.kind == BlockKind::Raw {
            continue;
        }

        let Some(extra_separator) = inter_block_extra_separator_range(source, &block, next) else {
            continue;
        };

        materialized
            .last_mut()
            .expect("current block should exist before materializing separator")
            .byte_range
            .end = extra_separator.start;
        materialized.push(BlockProjection {
            id: 0,
            kind: BlockKind::Raw,
            byte_range: extra_separator.clone(),
            content_range: extra_separator.start..extra_separator.start,
            cursor_anchor_policy: CursorAnchorPolicy::Clamp,
            can_code_edit: false,
        });
    }

    *blocks = materialized;
}

fn materialize_trailing_empty_block(blocks: &mut Vec<BlockProjection>, source: &Rope) {
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
    if !is_trailing_block_separator(&previous_text, &trailing_text) {
        return;
    }

    let Some(structural_len) = structural_separator_len(&previous_text, &trailing_text) else {
        return;
    };
    let separator_end = trailing.byte_range.start + structural_len;
    blocks.pop();
    if let Some(previous) = blocks.last_mut() {
        previous.byte_range.end = separator_end;
    }
    blocks.push(BlockProjection {
        id: 0,
        kind: BlockKind::Raw,
        byte_range: separator_end..source.len_bytes(),
        content_range: separator_end..separator_end,
        cursor_anchor_policy: CursorAnchorPolicy::Clamp,
        can_code_edit: false,
    });
}

fn inter_block_extra_separator_range(
    source: &Rope,
    previous: &BlockProjection,
    next: &BlockProjection,
) -> Option<Range<usize>> {
    let separator_end = cmp::min(previous.byte_range.end, next.byte_range.start);
    if previous.content_range.end >= separator_end {
        return None;
    }

    let separator_range = previous.content_range.end..separator_end;
    let separator_text = source_text(source, separator_range.clone());
    if separator_text.is_empty()
        || !separator_text
            .trim_matches([' ', '\t', '\r', '\n'])
            .is_empty()
    {
        return None;
    }

    let structural_len = structural_separator_len(
        &source_text(source, previous.content_range.clone()),
        &separator_text,
    )?;
    if structural_len >= separator_text.len() {
        return None;
    }

    Some(separator_range.start + structural_len..separator_range.end)
}

fn assign_block_ids(
    blocks: &mut [BlockProjection],
    previous_signatures: &[BlockSignature],
    source: &Rope,
    next_block_id: &mut u64,
) {
    if !previous_signatures.is_empty() {
        let next_signatures = blocks
            .iter()
            .map(|block| block_signature(source, block))
            .collect::<Vec<_>>();

        let mut previous_prefix = 0usize;
        let mut next_prefix = 0usize;
        while previous_prefix < previous_signatures.len()
            && next_prefix < next_signatures.len()
            && same_block_signature(
                &previous_signatures[previous_prefix],
                &next_signatures[next_prefix],
            )
        {
            blocks[next_prefix].id = previous_signatures[previous_prefix].id;
            previous_prefix += 1;
            next_prefix += 1;
        }

        let mut previous_suffix = previous_signatures.len();
        let mut next_suffix = next_signatures.len();
        while previous_suffix > previous_prefix
            && next_suffix > next_prefix
            && same_block_signature(
                &previous_signatures[previous_suffix - 1],
                &next_signatures[next_suffix - 1],
            )
        {
            previous_suffix -= 1;
            next_suffix -= 1;
            blocks[next_suffix].id = previous_signatures[previous_suffix].id;
        }

        for (block, previous_block) in blocks[next_prefix..next_suffix]
            .iter_mut()
            .zip(previous_signatures[previous_prefix..previous_suffix].iter())
        {
            block.id = previous_block.id;
        }
    }

    for block in blocks.iter_mut().filter(|block| block.id == 0) {
        block.id = take_next_block_id(next_block_id);
    }
}

fn block_signature(source: &Rope, block: &BlockProjection) -> BlockSignature {
    let mut hasher = DefaultHasher::new();
    for chunk in source.byte_slice(block.byte_range.clone()).chunks() {
        chunk.as_bytes().hash(&mut hasher);
    }

    BlockSignature {
        id: block.id,
        kind: block.kind.clone(),
        byte_len: block.byte_range.end.saturating_sub(block.byte_range.start),
        span_hash: hasher.finish(),
        cursor_anchor_policy: block.cursor_anchor_policy,
        can_code_edit: block.can_code_edit,
    }
}

fn same_block_signature(previous: &BlockSignature, current: &BlockSignature) -> bool {
    previous.kind == current.kind
        && previous.byte_len == current.byte_len
        && previous.span_hash == current.span_hash
        && previous.cursor_anchor_policy == current.cursor_anchor_policy
        && previous.can_code_edit == current.can_code_edit
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

fn projection_invariants_hold(blocks: &[BlockProjection], source: &Rope) -> bool {
    if blocks.is_empty() {
        return false;
    }

    let len = source.len_bytes();
    if blocks[0].byte_range.start != 0 {
        return false;
    }

    for (index, block) in blocks.iter().enumerate() {
        if block.byte_range.start > block.byte_range.end
            || block.byte_range.end > len
            || block.content_range.start < block.byte_range.start
            || block.content_range.start > block.content_range.end
            || block.content_range.end > block.byte_range.end
        {
            return false;
        }

        if let Some(next) = blocks.get(index + 1) {
            if block.byte_range.end != next.byte_range.start {
                return false;
            }
        }
    }

    blocks
        .last()
        .map(|block| block.byte_range.end == len)
        .unwrap_or(false)
}

fn structural_separator_len(previous_text: &str, separator_text: &str) -> Option<usize> {
    let mut required_newlines = 2usize.saturating_sub(trailing_newline_count(previous_text));
    if required_newlines == 0 {
        return Some(0);
    }

    let mut consumed = 0usize;
    for byte in separator_text.bytes() {
        consumed += 1;
        if byte == b'\n' {
            required_newlines -= 1;
            if required_newlines == 0 {
                return Some(consumed);
            }
        }
    }

    None
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

#[cfg(test)]
mod tests {
    use super::*;
    use markdown::{ParseOptions, mdast::Node, to_mdast};

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct ProjectionSummary {
        kind: BlockKind,
        text: String,
        trailing: String,
        cursor_anchor_policy: CursorAnchorPolicy,
        can_code_edit: bool,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct LegacyBlockProjection {
        kind: BlockKind,
        byte_range: Range<usize>,
        content_range: Range<usize>,
        cursor_anchor_policy: CursorAnchorPolicy,
        can_code_edit: bool,
    }

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
    fn materializes_editable_inter_block_empty_block_after_extra_separator() {
        let doc = DocumentBuffer::from_text("First\n\n\n\nSecond");

        assert_eq!(doc.blocks.len(), 3);
        assert_eq!(doc.blocks[0].kind, BlockKind::Paragraph);
        assert_eq!(doc.blocks[0].byte_range, 0..7);
        assert_eq!(doc.block_trailing_text(&doc.blocks[0]), "\n\n");
        assert_eq!(doc.blocks[1].kind, BlockKind::Raw);
        assert_eq!(doc.blocks[1].byte_range, 7..9);
        assert_eq!(doc.blocks[1].content_range, 7..7);
        assert_eq!(doc.block_text(&doc.blocks[1]), "");
        assert_eq!(doc.block_span_text(&doc.blocks[1]), "\n\n");
        assert_eq!(doc.block_trailing_text(&doc.blocks[1]), "\n\n");
        assert_eq!(doc.blocks[2].kind, BlockKind::Paragraph);
        assert_eq!(doc.block_text(&doc.blocks[2]), "Second");
    }

    #[test]
    fn does_not_materialize_inter_block_empty_block_for_standard_separator() {
        let doc = DocumentBuffer::from_text("- item\n\nNext");

        assert_eq!(doc.blocks.len(), 2);
        assert_eq!(doc.blocks[0].kind, BlockKind::List);
        assert_eq!(doc.blocks[1].kind, BlockKind::Paragraph);
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
    fn preserves_neighbor_ids_for_single_character_edit_inside_paragraph() {
        let mut doc =
            DocumentBuffer::from_text("# Title\n\nAlpha beta\n\n```rs\nfn main() {}\n```\n");
        let heading = doc.blocks[0].clone();
        let paragraph = doc.blocks[1].clone();
        let code = doc.blocks[2].clone();
        let insert_at = paragraph.content_range.start + 5;

        doc.replace_range(insert_at..insert_at, "!");

        assert_eq!(doc.blocks[0].id, heading.id);
        assert_eq!(doc.blocks[1].id, paragraph.id);
        assert_eq!(doc.blocks[2].id, code.id);
        assert_eq!(doc.block_text(&doc.blocks[1]), "Alpha! beta");
    }

    #[test]
    fn reparses_list_and_blockquote_blocks_without_losing_neighbors() {
        let mut doc = DocumentBuffer::from_text("- item\n\n> quote\n\nTail");
        let list = doc.blocks[0].clone();
        let quote_id = doc.blocks[1].id;
        let tail_id = doc.blocks[2].id;

        doc.replace_range(list.content_range.end..list.content_range.end, "\n- second");
        let quote = doc
            .block_by_id(quote_id)
            .cloned()
            .expect("quote block after list edit");
        doc.replace_range(quote.byte_range, "> quote\n> nested\n\n");

        assert_eq!(doc.blocks[0].id, list.id);
        assert_eq!(doc.blocks[1].id, quote_id);
        assert_eq!(doc.blocks[2].id, tail_id);
        assert_eq!(doc.block_text(&doc.blocks[2]), "Tail");
    }

    #[test]
    fn reparses_fenced_code_block_and_preserves_language() {
        let mut doc = DocumentBuffer::from_text("Before\n\n```rust\nfn main() {}\n```\n\nAfter");
        let code = doc
            .blocks
            .iter()
            .find(|block| matches!(block.kind, BlockKind::CodeFence { .. }))
            .cloned()
            .expect("code block");

        doc.replace_range(
            code.content_range.start..code.content_range.end,
            "```rust\nfn main() {\n    println!(\"hi\");\n}\n```",
        );

        let updated = doc
            .blocks
            .iter()
            .find(|block| block.id == code.id)
            .expect("updated code block");
        assert!(matches!(
            updated.kind,
            BlockKind::CodeFence {
                language: Some(ref language)
            } if language == "rust"
        ));
        assert!(doc.block_text(updated).contains("println!"));
    }

    #[test]
    fn maintains_projection_invariants_across_incremental_edits() {
        let mut doc = DocumentBuffer::from_text("# Title\n\n- item\n\n> quote\n\nTail");

        doc.replace_range(7..7, "\n");
        doc.replace_range(10..10, "second");
        doc.replace_range(doc.len()..doc.len(), "\n\n```rs\nfn main() {}\n```");

        assert!(projection_invariants_hold(doc.blocks(), &doc.source));
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
    fn applies_transactions_and_keeps_unrelated_content() {
        let mut doc = DocumentBuffer::from_text("# Title\n\nParagraph\n");
        let paragraph = doc
            .blocks
            .iter()
            .find(|block| block.kind == BlockKind::Paragraph)
            .cloned()
            .expect("paragraph");

        let applied = doc.apply_transaction(Transaction::Replace {
            range: paragraph.byte_range,
            replacement: "Updated paragraph\n".to_string(),
        });

        assert_eq!(applied.before_text, "Paragraph");
        assert_eq!(applied.after_text, "Updated paragraph\n");
        let paragraph = doc
            .blocks
            .iter()
            .find(|block| block.kind == BlockKind::Paragraph)
            .expect("paragraph after replace");
        assert_eq!(doc.block_text(paragraph), "Updated paragraph");
        assert!(doc.text().contains("# Title"));
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

    #[test]
    fn materializes_extra_trailing_separator_into_editable_empty_block() {
        let doc = DocumentBuffer::from_text("First\n\n\n");

        assert_eq!(doc.blocks.len(), 2);
        assert_eq!(doc.blocks[0].kind, BlockKind::Paragraph);
        assert_eq!(doc.block_trailing_text(&doc.blocks[0]), "\n\n");
        assert_eq!(doc.blocks[1].kind, BlockKind::Raw);
        assert_eq!(doc.blocks[1].content_range, 7..7);
        assert_eq!(doc.blocks[1].byte_range, 7..8);
        assert_eq!(doc.block_span_text(&doc.blocks[1]), "\n");
        assert_eq!(doc.block_trailing_text(&doc.blocks[1]), "\n");
    }

    #[test]
    fn recovers_incomplete_heading_without_trailing_newline() {
        let doc = DocumentBuffer::from_text("# Title");

        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(
            doc.blocks[0].kind,
            BlockKind::Heading { depth: 1 }
        ));
        assert_eq!(doc.block_text(&doc.blocks[0]), "# Title");
    }

    #[test]
    fn merges_incomplete_list_continuation_into_single_block() {
        let doc = DocumentBuffer::from_text("- item\n- ");

        assert_eq!(doc.blocks.len(), 1);
        assert_eq!(doc.blocks[0].kind, BlockKind::List);
        assert_eq!(doc.block_text(&doc.blocks[0]), "- item\n- ");
    }

    #[test]
    fn rebuilds_fresh_syntax_tree_from_current_text() {
        let mut doc = DocumentBuffer::from_text("# Title");
        doc.replace_range(7..7, "\n\n");
        doc.replace_range(9..9, "Tail");

        let reloaded = DocumentBuffer::from_text(doc.text());
        assert_eq!(projection_summary(&doc), projection_summary(&reloaded));
    }

    #[test]
    fn regression_corpus_matches_legacy_projection_for_supported_shapes() {
        // Editing-recovery cases like incomplete EOF headings and half-typed list items are
        // intentionally excluded here because the legacy parser only modeled stable shapes.
        let corpus = [
            ("paragraph", "Alpha\n\nBeta"),
            ("heading", "# Title\n\nBody"),
            ("list_task_list", "- [ ] task\n- item\n\nTail"),
            ("blockquote", "> quote\n> nested\n\nTail"),
            ("table", "| a | b |\n| - | - |\n| 1 | 2 |\n\nTail"),
            ("fenced_code", "```rust\nfn main() {}\n```\n\nTail"),
            ("html_block", "<div>\nhi\n</div>\n\nTail"),
            ("extra_blank_separators", "First\n\n\n\nSecond"),
            ("trailing_empty_block", "First\n\n"),
        ];

        for (name, text) in corpus {
            assert_eq!(
                projection_summary(&DocumentBuffer::from_text(text)),
                legacy_projection_summary(text),
                "corpus case: {name}"
            );
        }
    }

    fn projection_summary(doc: &DocumentBuffer) -> Vec<ProjectionSummary> {
        doc.blocks()
            .iter()
            .map(|block| ProjectionSummary {
                kind: block.kind.clone(),
                text: doc.block_text(block),
                trailing: doc.block_trailing_text(block),
                cursor_anchor_policy: block.cursor_anchor_policy,
                can_code_edit: block.can_code_edit,
            })
            .collect()
    }

    fn legacy_projection_summary(source: &str) -> Vec<ProjectionSummary> {
        legacy_parse_blocks(source)
            .into_iter()
            .map(|block| ProjectionSummary {
                kind: block.kind,
                text: source
                    .get(block.content_range.clone())
                    .unwrap_or_default()
                    .to_string(),
                trailing: source
                    .get(block.content_range.end..block.byte_range.end)
                    .unwrap_or_default()
                    .to_string(),
                cursor_anchor_policy: block.cursor_anchor_policy,
                can_code_edit: block.can_code_edit,
            })
            .collect()
    }

    fn legacy_parse_blocks(source: &str) -> Vec<LegacyBlockProjection> {
        if source.is_empty() {
            return vec![LegacyBlockProjection {
                kind: BlockKind::Raw,
                byte_range: 0..0,
                content_range: 0..0,
                cursor_anchor_policy: CursorAnchorPolicy::Clamp,
                can_code_edit: false,
            }];
        }

        let tree = to_mdast(source, &ParseOptions::gfm()).ok();
        let Some(Node::Root(root)) = tree else {
            return vec![LegacyBlockProjection {
                kind: BlockKind::Raw,
                byte_range: 0..source.len(),
                content_range: 0..source.len(),
                cursor_anchor_policy: CursorAnchorPolicy::Clamp,
                can_code_edit: false,
            }];
        };

        if root.children.is_empty() {
            return vec![LegacyBlockProjection {
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
                blocks.push(LegacyBlockProjection {
                    kind: BlockKind::Raw,
                    byte_range: cursor..start,
                    content_range: cursor..start,
                    cursor_anchor_policy: CursorAnchorPolicy::Clamp,
                    can_code_edit: false,
                });
            }

            blocks.push(LegacyBlockProjection {
                kind: legacy_block_kind(node),
                byte_range: start..span_end,
                content_range: start..content_end,
                cursor_anchor_policy: legacy_cursor_policy(node),
                can_code_edit: matches!(node, Node::Code(_)),
            });
            cursor = span_end;
        }

        if cursor < source.len() {
            blocks.push(LegacyBlockProjection {
                kind: BlockKind::Raw,
                byte_range: cursor..source.len(),
                content_range: cursor..source.len(),
                cursor_anchor_policy: CursorAnchorPolicy::Clamp,
                can_code_edit: false,
            });
        }

        if blocks.is_empty() {
            blocks.push(LegacyBlockProjection {
                kind: BlockKind::Raw,
                byte_range: 0..source.len(),
                content_range: 0..source.len(),
                cursor_anchor_policy: CursorAnchorPolicy::Clamp,
                can_code_edit: false,
            });
        }

        legacy_materialize_inter_block_empty_blocks(&mut blocks, source);
        legacy_materialize_trailing_empty_block(&mut blocks, source);

        blocks
    }

    fn legacy_materialize_inter_block_empty_blocks(
        blocks: &mut Vec<LegacyBlockProjection>,
        source: &str,
    ) {
        if blocks.len() < 2 {
            return;
        }

        let mut materialized = Vec::with_capacity(blocks.len());
        for (index, block) in blocks.iter().cloned().enumerate() {
            materialized.push(block.clone());

            let Some(next) = blocks.get(index + 1) else {
                continue;
            };
            if block.kind == BlockKind::Raw || next.kind == BlockKind::Raw {
                continue;
            }

            let Some(extra_separator) =
                legacy_inter_block_extra_separator_range(source, &block, next)
            else {
                continue;
            };

            materialized
                .last_mut()
                .expect("current block should exist before materializing separator")
                .byte_range
                .end = extra_separator.start;
            materialized.push(LegacyBlockProjection {
                kind: BlockKind::Raw,
                byte_range: extra_separator.clone(),
                content_range: extra_separator.start..extra_separator.start,
                cursor_anchor_policy: CursorAnchorPolicy::Clamp,
                can_code_edit: false,
            });
        }

        *blocks = materialized;
    }

    fn legacy_materialize_trailing_empty_block(
        blocks: &mut Vec<LegacyBlockProjection>,
        source: &str,
    ) {
        let Some(trailing) = blocks.last().cloned() else {
            return;
        };
        if trailing.kind != BlockKind::Raw {
            return;
        }
        if blocks.len() < 2 || blocks[blocks.len() - 2].kind == BlockKind::Raw {
            return;
        }

        let previous_text =
            legacy_source_text(source, blocks[blocks.len() - 2].content_range.clone());
        let trailing_text = legacy_source_text(source, trailing.byte_range.clone());
        if !is_trailing_block_separator(previous_text, trailing_text) {
            return;
        }

        let separator_end = trailing.byte_range.end;
        blocks.pop();
        if let Some(previous) = blocks.last_mut() {
            previous.byte_range.end = separator_end;
        }
        blocks.push(LegacyBlockProjection {
            kind: BlockKind::Raw,
            byte_range: source.len()..source.len(),
            content_range: source.len()..source.len(),
            cursor_anchor_policy: CursorAnchorPolicy::Clamp,
            can_code_edit: false,
        });
    }

    fn legacy_inter_block_extra_separator_range(
        source: &str,
        previous: &LegacyBlockProjection,
        next: &LegacyBlockProjection,
    ) -> Option<Range<usize>> {
        let separator_end = cmp::min(previous.byte_range.end, next.byte_range.start);
        if previous.content_range.end >= separator_end {
            return None;
        }

        let separator_range = previous.content_range.end..separator_end;
        let separator_text = legacy_source_text(source, separator_range.clone());
        if separator_text.is_empty()
            || !separator_text
                .trim_matches([' ', '\t', '\r', '\n'])
                .is_empty()
        {
            return None;
        }

        let structural_len = structural_separator_len(
            legacy_source_text(source, previous.content_range.clone()),
            separator_text,
        )?;
        if structural_len >= separator_text.len() {
            return None;
        }

        Some(separator_range.start + structural_len..separator_range.end)
    }

    fn legacy_source_text(source: &str, range: Range<usize>) -> &str {
        source.get(range).unwrap_or_default()
    }

    fn legacy_block_kind(node: &Node) -> BlockKind {
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

    fn legacy_cursor_policy(node: &Node) -> CursorAnchorPolicy {
        if matches!(node, Node::Code(_)) {
            CursorAnchorPolicy::PreserveColumn
        } else {
            CursorAnchorPolicy::Clamp
        }
    }
}
