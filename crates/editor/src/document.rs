use std::{
    cmp, fs,
    ops::Range,
    path::{Path, PathBuf},
    time::SystemTime,
};

use anyhow::{Context as _, Result};
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
pub struct BlockSpan {
    pub id: u64,
    pub kind: BlockKind,
    pub byte_range: Range<usize>,
    pub cursor_anchor_policy: CursorAnchorPolicy,
    pub can_code_edit: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictState {
    Clean,
    Conflict {
        disk_text: String,
        observed_at: Option<SystemTime>,
    },
}

#[derive(Debug)]
pub struct DocumentState {
    pub path: Option<PathBuf>,
    suggested_path: Option<PathBuf>,
    pub source: Rope,
    pub dirty: bool,
    pub saving: bool,
    pub conflict: ConflictState,
    pub blocks: Vec<BlockSpan>,
    pub parse_version: u64,
    modified_at: Option<SystemTime>,
}

impl DocumentState {
    pub fn new_empty(path: Option<PathBuf>, suggested_path: Option<PathBuf>) -> Self {
        let mut this = Self {
            path,
            suggested_path,
            source: Rope::new(),
            dirty: false,
            saving: false,
            conflict: ConflictState::Clean,
            blocks: Vec::new(),
            parse_version: 0,
            modified_at: None,
        };
        this.reparse();
        this
    }

    pub fn from_disk(path: PathBuf) -> Result<Self> {
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let modified_at = file_modified_at(&path);
        let mut this = Self {
            path: Some(path.clone()),
            suggested_path: Some(path),
            source: Rope::from_str(&text),
            dirty: false,
            saving: false,
            conflict: ConflictState::Clean,
            blocks: Vec::new(),
            parse_version: 0,
            modified_at,
        };
        this.reparse();
        Ok(this)
    }

    #[cfg(test)]
    pub fn from_text(
        path: Option<PathBuf>,
        suggested_path: Option<PathBuf>,
        text: impl AsRef<str>,
    ) -> Self {
        let mut this = Self {
            path,
            suggested_path,
            source: Rope::from_str(text.as_ref()),
            dirty: false,
            saving: false,
            conflict: ConflictState::Clean,
            blocks: Vec::new(),
            parse_version: 0,
            modified_at: None,
        };
        this.reparse();
        this
    }

    pub fn suggested_path(&self) -> Option<&PathBuf> {
        self.suggested_path.as_ref().or(self.path.as_ref())
    }

    pub fn display_name(&self) -> String {
        if let Some(path) = &self.path {
            return path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Untitled")
                .to_string();
        }

        if let Some(path) = &self.suggested_path {
            return path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Untitled")
                .to_string();
        }

        "Untitled.md".to_string()
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

    pub fn block_index_by_id(&self, block_id: u64) -> Option<usize> {
        self.blocks.iter().position(|block| block.id == block_id)
    }

    pub fn block_by_id(&self, block_id: u64) -> Option<&BlockSpan> {
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

    pub fn block_text(&self, block: &BlockSpan) -> String {
        self.text_for_range(block.byte_range.clone())
    }

    pub fn text_for_range(&self, range: Range<usize>) -> String {
        self.source
            .get_byte_slice(range)
            .expect("block byte range should align to UTF-8 boundaries")
            .to_string()
    }

    pub fn replace_range(&mut self, range: Range<usize>, replacement: &str) {
        let mut text = self.text();
        let start = cmp::min(range.start, text.len());
        let end = cmp::min(range.end, text.len());
        assert!(
            text.is_char_boundary(start) && text.is_char_boundary(end),
            "document byte range should align to UTF-8 boundaries"
        );
        text.replace_range(start..end, replacement);
        self.source = Rope::from_str(&text);
        self.dirty = true;
        self.saving = false;
        self.reparse();
    }

    pub fn overwrite_from_disk_text(
        &mut self,
        path: PathBuf,
        text: impl AsRef<str>,
        modified_at: Option<SystemTime>,
    ) {
        self.path = Some(path.clone());
        self.suggested_path = Some(path);
        self.source = Rope::from_str(text.as_ref());
        self.dirty = false;
        self.saving = false;
        self.conflict = ConflictState::Clean;
        self.modified_at = modified_at;
        self.reparse();
    }

    pub fn mark_conflict(&mut self, disk_text: String, observed_at: Option<SystemTime>) {
        self.conflict = ConflictState::Conflict {
            disk_text,
            observed_at,
        };
    }

    pub fn keep_current_version(&mut self) {
        if let ConflictState::Conflict { observed_at, .. } = self.conflict.clone() {
            self.modified_at = observed_at;
        }
        self.conflict = ConflictState::Clean;
    }

    pub fn save_now(&mut self) -> Result<()> {
        let path = self
            .path
            .clone()
            .or_else(|| self.suggested_path.clone())
            .context("cannot save without a target path")?;

        self.saving = true;
        let text = self.text();
        if let Err(err) = fs::write(&path, text.as_bytes())
            .with_context(|| format!("failed to write {}", path.display()))
        {
            self.saving = false;
            return Err(err);
        }
        self.path = Some(path.clone());
        self.suggested_path = Some(path.clone());
        self.modified_at = file_modified_at(&path);
        self.dirty = false;
        self.saving = false;
        self.conflict = ConflictState::Clean;
        Ok(())
    }

    pub fn set_path(&mut self, path: PathBuf) {
        self.path = Some(path.clone());
        self.suggested_path = Some(path);
    }

    pub fn has_same_disk_timestamp(&self, path: &Path) -> bool {
        file_modified_at(path) == self.modified_at
    }

    fn reparse(&mut self) {
        self.parse_version = self.parse_version.wrapping_add(1);
        self.blocks = parse_blocks(&self.text(), self.parse_version);
        if self.blocks.is_empty() {
            self.blocks.push(BlockSpan {
                id: make_block_id(self.parse_version, 0),
                kind: BlockKind::Raw,
                byte_range: 0..0,
                cursor_anchor_policy: CursorAnchorPolicy::Clamp,
                can_code_edit: false,
            });
        }
    }
}

fn make_block_id(parse_version: u64, index: usize) -> u64 {
    (parse_version << 32) | index as u64
}

fn parse_blocks(source: &str, parse_version: u64) -> Vec<BlockSpan> {
    if source.is_empty() {
        return vec![BlockSpan {
            id: make_block_id(parse_version, 0),
            kind: BlockKind::Raw,
            byte_range: 0..0,
            cursor_anchor_policy: CursorAnchorPolicy::Clamp,
            can_code_edit: false,
        }];
    }

    let tree = to_mdast(source, &ParseOptions::gfm()).ok();
    let Some(Node::Root(root)) = tree else {
        return vec![BlockSpan {
            id: make_block_id(parse_version, 0),
            kind: BlockKind::Raw,
            byte_range: 0..source.len(),
            cursor_anchor_policy: CursorAnchorPolicy::Clamp,
            can_code_edit: false,
        }];
    };

    if root.children.is_empty() {
        return vec![BlockSpan {
            id: make_block_id(parse_version, 0),
            kind: BlockKind::Raw,
            byte_range: 0..source.len(),
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
        let end = root
            .children
            .get(index + 1)
            .and_then(Node::position)
            .map(|pos| cmp::min(pos.start.offset, source.len()))
            .unwrap_or_else(|| cmp::min(position.end.offset, source.len()));
        let span_end = cmp::max(cmp::min(position.end.offset, source.len()), end);

        if start > cursor {
            blocks.push(BlockSpan {
                id: make_block_id(parse_version, blocks.len()),
                kind: BlockKind::Raw,
                byte_range: cursor..start,
                cursor_anchor_policy: CursorAnchorPolicy::Clamp,
                can_code_edit: false,
            });
        }

        blocks.push(BlockSpan {
            id: make_block_id(parse_version, blocks.len()),
            kind: block_kind(node),
            byte_range: start..span_end,
            cursor_anchor_policy: cursor_policy(node),
            can_code_edit: matches!(node, Node::Code(_)),
        });
        cursor = span_end;
    }

    if cursor < source.len() {
        blocks.push(BlockSpan {
            id: make_block_id(parse_version, blocks.len()),
            kind: BlockKind::Raw,
            byte_range: cursor..source.len(),
            cursor_anchor_policy: CursorAnchorPolicy::Clamp,
            can_code_edit: false,
        });
    }

    if blocks.is_empty() {
        blocks.push(BlockSpan {
            id: make_block_id(parse_version, 0),
            kind: BlockKind::Raw,
            byte_range: 0..source.len(),
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

fn file_modified_at(path: &Path) -> Option<SystemTime> {
    fs::metadata(path)
        .ok()
        .and_then(|meta| meta.modified().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_empty_document_into_single_raw_block() {
        let doc = DocumentState::new_empty(None, None);
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
        let doc = DocumentState::from_text(None, None, text);
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
        let doc = DocumentState::from_text(None, None, text);
        assert_eq!(doc.blocks[0].kind, BlockKind::Raw);
        assert_eq!(doc.block_text(&doc.blocks[0]), "\n\n");
        assert!(matches!(doc.blocks[1].kind, BlockKind::Heading { .. }));
    }

    #[test]
    fn reparses_after_splice_and_tracks_block_merges() {
        let mut doc = DocumentState::from_text(None, None, "# Title\n\nParagraph\n");
        let first_parse = doc.parse_version;
        let paragraph = doc
            .blocks
            .iter()
            .find(|block| block.kind == BlockKind::Paragraph)
            .cloned()
            .expect("paragraph block");
        doc.replace_range(paragraph.byte_range, "Paragraph\n\n## Child\n");
        assert!(doc.parse_version > first_parse);
        assert!(
            doc.blocks
                .iter()
                .any(|block| matches!(block.kind, BlockKind::Heading { depth: 2 }))
        );
    }

    #[test]
    fn maps_offsets_back_to_current_blocks() {
        let doc =
            DocumentState::from_text(None, None, "# A\n\nParagraph\n\n```rs\nfn main() {}\n```\n");
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
    fn handles_multibyte_block_ranges_and_splices() {
        let mut doc = DocumentState::from_text(None, None, "# 鏍囬\n\n娈佃惤馃檪\n");
        let paragraph = doc
            .blocks
            .iter()
            .find(|block| block.kind == BlockKind::Paragraph)
            .cloned()
            .expect("paragraph");

        assert_eq!(doc.block_text(&paragraph), "娈佃惤馃檪");

        doc.replace_range(paragraph.byte_range, "鏇存柊馃檪\n");

        let paragraph = doc
            .blocks
            .iter()
            .find(|block| block.kind == BlockKind::Paragraph)
            .expect("paragraph after replace");
        assert_eq!(doc.block_text(paragraph), "鏇存柊馃檪");
        assert!(doc.text().contains("# 鏍囬"));
    }
}
