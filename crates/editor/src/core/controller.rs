use std::{
    cmp, fs,
    ops::Range,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use anyhow::{Context as _, Result};

use super::{
    display_map::{DisplayMap, HiddenSyntaxPolicy},
    document::{
        BlockKind, BlockProjection, DocumentBuffer, SelectionAffinity, SelectionState,
        Transaction,
    },
    table::{TableCellRef, TableColumnAlignment, TableModel, TableNavDirection},
    text_ops::{
        AutoFormatAction, adjust_block_markup, adjust_list_markup_at_cursor,
        adjust_quoted_list_markup_at_cursor, adjust_selected_list_markup,
        adjust_selected_quoted_list_markup, byte_offset_for_line_column, clamp_to_char_boundary,
        compute_document_diff, count_document_words, detect_auto_format,
        line_column_for_byte_offset, opening_fence_marker, pipe_table_enter_transform,
        semantic_enter_transform, set_blockquote_markup, set_heading_markup, set_list_markup,
        set_task_list_markup,
    },
};

#[derive(Debug, Clone)]
pub enum DocumentSource {
    Empty {
        suggested_path: Option<PathBuf>,
    },
    Text {
        path: Option<PathBuf>,
        suggested_path: Option<PathBuf>,
        text: String,
        modified_at: Option<SystemTime>,
    },
}

impl DocumentSource {
    pub fn from_disk(path: PathBuf) -> Result<Self> {
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let modified_at = file_modified_at(&path);
        Ok(Self::Text {
            path: Some(path.clone()),
            suggested_path: Some(path),
            text,
            modified_at,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SyncPolicy {
    pub autosave_delay: Duration,
}

impl Default for SyncPolicy {
    fn default() -> Self {
        Self {
            autosave_delay: Duration::from_millis(700),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictState {
    Clean,
    Conflict {
        disk_text: String,
        observed_at: Option<SystemTime>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncState {
    Clean,
    Dirty,
    Saving,
    Conflict,
    Missing,
}

#[derive(Debug, Clone)]
pub struct BlockSnapshot {
    pub id: u64,
    pub kind: BlockKind,
    pub text: String,
    pub can_code_edit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaretPosition {
    pub byte: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorViewMode {
    LivePreview,
    Source,
}

impl Default for EditorViewMode {
    fn default() -> Self {
        Self::LivePreview
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutlineItem {
    pub block_id: u64,
    pub depth: u8,
    pub title: String,
    pub source_offset: usize,
}

#[derive(Debug, Clone)]
pub struct EditorSnapshot {
    pub path: Option<PathBuf>,
    pub suggested_path: Option<PathBuf>,
    pub display_name: String,
    pub sync_state: SyncState,
    pub dirty: bool,
    pub saving: bool,
    pub has_conflict: bool,
    pub is_missing: bool,
    pub word_count: usize,
    pub status_message: String,
    pub document_text: String,
    pub selection: SelectionState,
    pub caret_position: CaretPosition,
    pub visible_selection: SelectionState,
    pub visible_caret_position: CaretPosition,
    pub display_map: DisplayMap,
    pub blocks: Vec<BlockSnapshot>,
    pub outline: Vec<OutlineItem>,
    pub view_mode: EditorViewMode,
    pub find_matches: Vec<std::ops::Range<usize>>,
    pub active_find_index: Option<usize>,
}

impl EditorSnapshot {
    pub fn block_by_id(&self, block_id: u64) -> Option<&BlockSnapshot> {
        self.blocks.iter().find(|block| block.id == block_id)
    }

    pub fn block_index_by_id(&self, block_id: u64) -> Option<usize> {
        self.blocks.iter().position(|block| block.id == block_id)
    }
}

#[derive(Debug, Clone, Default)]
pub struct EditorEffects {
    pub changed: bool,
    pub selection_changed: bool,
    pub reload_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub enum FileSyncEvent {
    Changed(PathBuf),
    Removed(PathBuf),
    Relocated { from: PathBuf, to: PathBuf },
    Unknown,
}

#[derive(Debug, Clone)]
pub enum EditCommand {
    SyncDocumentState {
        text: String,
        selection: SelectionState,
    },
    SetSelection {
        selection: SelectionState,
    },
    ReplaceSelection {
        text: String,
    },
    InsertBreak {
        plain: bool,
    },
    ToggleInlineMarkup {
        before: String,
        after: String,
    },
    InsertLink,
    InsertImage,
    Indent,
    Outdent,
    MoveCaret {
        direction: isize,
        preferred_column: Option<usize>,
    },
    DeleteBackward,
    DeleteForward,
    DeleteSurroundingPair {
        before_len: usize,
        after_len: usize,
    },
    /// Toggle heading at the given depth (1–6). If the current block is already
    /// a heading at that depth, it reverts to a plain paragraph (depth 0).
    /// Passing depth 0 always converts to paragraph.
    ToggleHeading {
        depth: u8,
    },
    /// Toggle blockquote (`> `) prefix. If the block is already a blockquote, strips it.
    ToggleBlockquote,
    /// Toggle bullet list (`- `) prefix. If already a bullet list, strips it.
    ToggleBulletList,
    /// Toggle ordered list (`1. `) prefix. If already an ordered list, strips it.
    ToggleOrderedList,
    /// Convert the current block to a task list item with an unchecked checkbox.
    ToggleTaskList,
    /// Insert a thematic break (`---`). If the current block is an empty paragraph, replace it;
    /// otherwise insert after the current block and leave the cursor in the following empty paragraph.
    InsertHorizontalRule,
    /// Insert a fenced code block (` ``` \n\n``` `). If the current block is an empty paragraph,
    /// replace it; otherwise insert after the current block. The cursor lands on the empty line
    /// between the opening and closing fence markers.
    InsertCodeFence,
    InsertMermaidDiagram,
    /// Insert a small pipe table template and place the cursor in the first body cell.
    InsertTable,
    InsertInlineMath,
    InsertMathBlock,
    InsertHtmlBlock,
    InsertCallout,
    InsertToc,
    InsertFootnote,
    InsertFrontMatter,
    Undo,
    Redo,
    ReloadConflict,
    KeepCurrentConflict,
}

#[derive(Debug, Clone)]
struct EditHistoryEntry {
    before_range: Range<usize>,
    before_text: String,
    after_range: Range<usize>,
    after_text: String,
    selection_before: SelectionState,
    selection_after: SelectionState,
}

#[derive(Debug, Clone)]
struct FileSyncCoordinator {
    path: Option<PathBuf>,
    suggested_path: Option<PathBuf>,
    modified_at: Option<SystemTime>,
    baseline_text: String,
    dirty: bool,
    saving: bool,
    missing_on_disk: bool,
    conflict: ConflictState,
}

impl FileSyncCoordinator {
    fn new_empty(suggested_path: Option<PathBuf>) -> Self {
        Self {
            path: None,
            suggested_path,
            modified_at: None,
            baseline_text: String::new(),
            dirty: false,
            saving: false,
            missing_on_disk: false,
            conflict: ConflictState::Clean,
        }
    }

    fn from_text(
        path: Option<PathBuf>,
        suggested_path: Option<PathBuf>,
        text: String,
        modified_at: Option<SystemTime>,
    ) -> Self {
        Self {
            path,
            suggested_path,
            modified_at,
            baseline_text: text,
            dirty: false,
            saving: false,
            missing_on_disk: false,
            conflict: ConflictState::Clean,
        }
    }

    fn display_name(&self) -> String {
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

    fn sync_state(&self) -> SyncState {
        if matches!(self.conflict, ConflictState::Conflict { .. }) {
            SyncState::Conflict
        } else if self.missing_on_disk {
            SyncState::Missing
        } else if self.saving {
            SyncState::Saving
        } else if self.dirty {
            SyncState::Dirty
        } else {
            SyncState::Clean
        }
    }

    fn set_path(&mut self, path: PathBuf) {
        self.path = Some(path.clone());
        self.suggested_path = Some(path);
        self.missing_on_disk = false;
    }

    fn suggested_path(&self) -> Option<&PathBuf> {
        self.suggested_path.as_ref().or(self.path.as_ref())
    }

    fn current_dir(&self) -> Option<PathBuf> {
        self.suggested_path()
            .and_then(|path| path.parent().map(Path::to_path_buf))
    }

    fn mark_document_changed(&mut self, current_text: &str) {
        self.dirty = current_text != self.baseline_text;
        self.saving = false;
        self.missing_on_disk = false;
    }

    fn mark_saved(&mut self, path: PathBuf, current_text: String, modified_at: Option<SystemTime>) {
        self.path = Some(path.clone());
        self.suggested_path = Some(path);
        self.modified_at = modified_at;
        self.baseline_text = current_text;
        self.dirty = false;
        self.saving = false;
        self.missing_on_disk = false;
        self.conflict = ConflictState::Clean;
    }

    fn mark_loaded_from_disk(
        &mut self,
        path: PathBuf,
        text: String,
        modified_at: Option<SystemTime>,
    ) {
        self.path = Some(path.clone());
        self.suggested_path = Some(path);
        self.modified_at = modified_at;
        self.baseline_text = text;
        self.dirty = false;
        self.saving = false;
        self.missing_on_disk = false;
        self.conflict = ConflictState::Clean;
    }

    fn mark_conflict(&mut self, disk_text: String, observed_at: Option<SystemTime>) {
        self.conflict = ConflictState::Conflict {
            disk_text,
            observed_at,
        };
        self.saving = false;
    }

    fn keep_current_conflicted_version(&mut self) {
        if let ConflictState::Conflict { observed_at, .. } = self.conflict.clone() {
            self.modified_at = observed_at;
        }
        self.conflict = ConflictState::Clean;
        self.saving = false;
    }

    fn mark_missing(&mut self) {
        self.missing_on_disk = true;
        self.saving = false;
    }

    fn relocate(&mut self, to: PathBuf) {
        self.path = Some(to.clone());
        self.suggested_path = Some(to);
        self.missing_on_disk = false;
    }

    fn has_same_disk_timestamp(&self, modified_at: Option<SystemTime>) -> bool {
        self.modified_at == modified_at
    }
}

pub struct EditorController {
    sync_policy: SyncPolicy,
    document: DocumentBuffer,
    sync: FileSyncCoordinator,
    selection: SelectionState,
    status_message: String,
    undo_stack: Vec<EditHistoryEntry>,
    redo_stack: Vec<EditHistoryEntry>,
    view_mode: EditorViewMode,
    prev_display_map: std::cell::RefCell<Option<DisplayMap>>,
}

impl EditorController {
    pub fn new(source: DocumentSource, sync_policy: SyncPolicy) -> Self {
        let (document, sync) = match source {
            DocumentSource::Empty { suggested_path } => (
                DocumentBuffer::new_empty(),
                FileSyncCoordinator::new_empty(suggested_path),
            ),
            DocumentSource::Text {
                path,
                suggested_path,
                text,
                modified_at,
            } => (
                DocumentBuffer::from_text(text.clone()),
                FileSyncCoordinator::from_text(path, suggested_path, text, modified_at),
            ),
        };

        Self {
            sync_policy,
            document,
            sync,
            selection: SelectionState::collapsed(0),
            status_message: String::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            view_mode: EditorViewMode::LivePreview,
            prev_display_map: std::cell::RefCell::new(None),
        }
    }

    pub fn from_disk(path: PathBuf, sync_policy: SyncPolicy) -> Result<Self> {
        Ok(Self::new(DocumentSource::from_disk(path)?, sync_policy))
    }

    pub fn autosave_delay(&self) -> Duration {
        self.sync_policy.autosave_delay
    }

    pub(crate) fn navigate_table(&mut self, backwards: bool) -> EditorEffects {
        let Some(block) = self.current_block().cloned() else {
            return EditorEffects::default();
        };
        if block.kind != BlockKind::Table {
            return EditorEffects::default();
        }

        let direction = if backwards {
            TableNavDirection::Backward
        } else {
            TableNavDirection::Forward
        };
        self.navigate_table_from_block(&block, direction)
    }

    pub(crate) fn delete_table_row(&mut self) -> EditorEffects {
        let Some(block) = self.current_block().cloned() else {
            return EditorEffects::default();
        };
        if block.kind != BlockKind::Table {
            return EditorEffects::default();
        }

        let table = TableModel::parse(&self.document.block_text(&block));
        if table.is_empty() {
            return EditorEffects::default();
        }

        let local_cursor = self
            .selection
            .cursor()
            .saturating_sub(block.content_range.start);
        let Some(current_cell) = table
            .cell_ref_for_source_offset(local_cursor, self.selection.affinity)
            .or_else(|| table.first_cell())
        else {
            return EditorEffects::default();
        };

        let Some(replacement) = table.rebuild_markdown_without_row(current_cell.visible_row) else {
            return EditorEffects::default();
        };
        let rebuilt = TableModel::parse(&replacement);
        let target_cell = TableCellRef {
            visible_row: current_cell
                .visible_row
                .min(rebuilt.visible_row_count().saturating_sub(1)),
            column: current_cell
                .column
                .min(rebuilt.column_count().saturating_sub(1)),
        };
        let selection_after = table_cell_selection(&block, &rebuilt, target_cell);
        let trailing = self.document.block_trailing_text(&block);
        self.apply_edit(
            block.byte_range.clone(),
            format!("{}{}", replacement, trailing),
            selection_after,
            "Deleted table row",
        )
    }

    pub(crate) fn insert_table_row(&mut self) -> EditorEffects {
        let Some(block) = self.current_block().cloned() else {
            return EditorEffects::default();
        };
        if block.kind != BlockKind::Table {
            return EditorEffects::default();
        }

        let table = TableModel::parse(&self.document.block_text(&block));
        if table.is_empty() {
            return EditorEffects::default();
        }

        let local_cursor = self
            .selection
            .cursor()
            .saturating_sub(block.content_range.start);
        let Some(current_cell) = table
            .cell_ref_for_source_offset(local_cursor, self.selection.affinity)
            .or_else(|| table.first_cell())
        else {
            return EditorEffects::default();
        };

        let Some(replacement) =
            table.rebuild_markdown_with_inserted_row_after(current_cell.visible_row)
        else {
            return EditorEffects::default();
        };
        let rebuilt = TableModel::parse(&replacement);
        let target_cell = TableCellRef {
            visible_row: (current_cell.visible_row + 1)
                .min(rebuilt.visible_row_count().saturating_sub(1)),
            column: current_cell
                .column
                .min(rebuilt.column_count().saturating_sub(1)),
        };
        let selection_after = table_cell_selection(&block, &rebuilt, target_cell);
        let trailing = self.document.block_trailing_text(&block);
        self.apply_edit(
            block.byte_range.clone(),
            format!("{}{}", replacement, trailing),
            selection_after,
            "Inserted table row",
        )
    }

    pub(crate) fn insert_table_column(&mut self) -> EditorEffects {
        let Some(block) = self.current_block().cloned() else {
            return EditorEffects::default();
        };
        if block.kind != BlockKind::Table {
            return EditorEffects::default();
        }

        let table = TableModel::parse(&self.document.block_text(&block));
        if table.is_empty() {
            return EditorEffects::default();
        }

        let local_cursor = self
            .selection
            .cursor()
            .saturating_sub(block.content_range.start);
        let Some(current_cell) = table
            .cell_ref_for_source_offset(local_cursor, self.selection.affinity)
            .or_else(|| table.first_cell())
        else {
            return EditorEffects::default();
        };

        let Some(replacement) =
            table.rebuild_markdown_with_inserted_column_after(current_cell.column)
        else {
            return EditorEffects::default();
        };
        let rebuilt = TableModel::parse(&replacement);
        let target_cell = TableCellRef {
            visible_row: current_cell
                .visible_row
                .min(rebuilt.visible_row_count().saturating_sub(1)),
            column: (current_cell.column + 1).min(rebuilt.column_count().saturating_sub(1)),
        };
        let selection_after = table_cell_selection(&block, &rebuilt, target_cell);
        let trailing = self.document.block_trailing_text(&block);
        self.apply_edit(
            block.byte_range.clone(),
            format!("{}{}", replacement, trailing),
            selection_after,
            "Inserted table column",
        )
    }

    pub(crate) fn delete_table_column(&mut self) -> EditorEffects {
        let Some(block) = self.current_block().cloned() else {
            return EditorEffects::default();
        };
        if block.kind != BlockKind::Table {
            return EditorEffects::default();
        }

        let table = TableModel::parse(&self.document.block_text(&block));
        if table.is_empty() {
            return EditorEffects::default();
        }

        let local_cursor = self
            .selection
            .cursor()
            .saturating_sub(block.content_range.start);
        let Some(current_cell) = table
            .cell_ref_for_source_offset(local_cursor, self.selection.affinity)
            .or_else(|| table.first_cell())
        else {
            return EditorEffects::default();
        };

        let Some(replacement) = table.rebuild_markdown_without_column(current_cell.column) else {
            return EditorEffects::default();
        };
        let rebuilt = TableModel::parse(&replacement);
        let target_cell = TableCellRef {
            visible_row: current_cell
                .visible_row
                .min(rebuilt.visible_row_count().saturating_sub(1)),
            column: current_cell
                .column
                .min(rebuilt.column_count().saturating_sub(1)),
        };
        let selection_after = table_cell_selection(&block, &rebuilt, target_cell);
        let trailing = self.document.block_trailing_text(&block);
        self.apply_edit(
            block.byte_range.clone(),
            format!("{}{}", replacement, trailing),
            selection_after,
            "Deleted table column",
        )
    }

    pub(crate) fn align_table_column(&mut self, alignment: TableColumnAlignment) -> EditorEffects {
        let Some(block) = self.current_block().cloned() else {
            return EditorEffects::default();
        };
        if block.kind != BlockKind::Table {
            return EditorEffects::default();
        }

        let table = TableModel::parse(&self.document.block_text(&block));
        if table.is_empty() {
            return EditorEffects::default();
        }

        let local_cursor = self
            .selection
            .cursor()
            .saturating_sub(block.content_range.start);
        let Some(current_cell) = table
            .cell_ref_for_source_offset(local_cursor, self.selection.affinity)
            .or_else(|| table.first_cell())
        else {
            return EditorEffects::default();
        };

        let Some(replacement) =
            table.rebuild_markdown_with_column_alignment(current_cell.column, alignment)
        else {
            return EditorEffects::default();
        };
        let rebuilt = TableModel::parse(&replacement);
        let selection_after = table_cell_selection(&block, &rebuilt, current_cell);
        let trailing = self.document.block_trailing_text(&block);
        self.apply_edit(
            block.byte_range.clone(),
            format!("{}{}", replacement, trailing),
            selection_after,
            "Aligned table column",
        )
    }

    pub(crate) fn exit_table(&mut self) -> EditorEffects {
        let Some(block) = self.current_block().cloned() else {
            return EditorEffects::default();
        };
        if block.kind != BlockKind::Table {
            return EditorEffects::default();
        }

        let Some(block_index) = self.document.block_index_by_id(block.id) else {
            return EditorEffects::default();
        };
        if let Some(next_block) = self.document.blocks().get(block_index + 1)
            && next_block.kind == BlockKind::Raw
            && next_block.content_range.is_empty()
        {
            return self
                .update_selection(SelectionState::collapsed(next_block.content_range.start));
        }

        let insertion_point = block.byte_range.end;
        let selection_after = if block.byte_range.end > block.content_range.end {
            insertion_point
        } else {
            insertion_point + 2
        };
        self.apply_edit(
            insertion_point..insertion_point,
            "\n\n".to_string(),
            SelectionState::collapsed(selection_after),
            "Exited table",
        )
    }
}

impl EditorController {
    pub fn snapshot(&self) -> EditorSnapshot {
        let document_text = self.document.text();
        let selection = clamp_selection_to_text(&document_text, self.selection.clone());
        let caret_byte = selection.cursor().min(document_text.len());
        let (line, column) = line_column_for_byte_offset(&document_text, caret_byte);
        let display_map = match self.view_mode {
            EditorViewMode::LivePreview => {
                let prev = self.prev_display_map.borrow().clone();
                let incremental = DisplayMap::from_document_incremental(
                    &self.document,
                    Some(&selection),
                    HiddenSyntaxPolicy::SelectionAware,
                    prev.as_ref(),
                );
                *self.prev_display_map.borrow_mut() = Some(incremental.clone());
                incremental
            }
            EditorViewMode::Source => {
                let prev = self.prev_display_map.borrow().clone();
                if let Some(prev_map) = prev.as_ref() {
                    use std::hash::{Hash, Hasher};
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    document_text.hash(&mut hasher);
                    let current_hash = hasher.finish();
                    let prev_hash = prev_map.blocks.first().map(|b| b.source_hash).unwrap_or(0);
                    if current_hash == prev_hash {
                        prev_map.clone()
                    } else {
                        let map = self.document.source_display_map();
                        *self.prev_display_map.borrow_mut() = Some(map.clone());
                        map
                    }
                } else {
                    let map = self.document.source_display_map();
                    *self.prev_display_map.borrow_mut() = Some(map.clone());
                    map
                }
            }
        };
        let mut visible_selection = display_map.source_selection_to_visible(&selection);
        let visible_text = display_map.visible_text.clone();
        let visible_caret_byte = visible_selection.cursor().min(visible_text.len());
        let (visible_line, visible_column) =
            line_column_for_byte_offset(&visible_text, visible_caret_byte);
        visible_selection.preferred_column = Some(visible_column);
        let blocks = self
            .document
            .blocks()
            .iter()
            .map(|block| BlockSnapshot {
                id: block.id,
                kind: block.kind.clone(),
                text: self.document.block_text(block),
                can_code_edit: block.can_code_edit,
            })
            .collect::<Vec<_>>();
        let outline = build_outline(&self.document);
        EditorSnapshot {
            path: self.sync.path.clone(),
            suggested_path: self.sync.suggested_path.clone(),
            display_name: self.sync.display_name(),
            sync_state: self.sync.sync_state(),
            dirty: self.sync.dirty,
            saving: self.sync.saving,
            has_conflict: matches!(self.sync.conflict, ConflictState::Conflict { .. }),
            is_missing: self.sync.missing_on_disk,
            word_count: count_document_words(&document_text),
            status_message: self.status_message.clone(),
            document_text,
            selection,
            caret_position: CaretPosition {
                byte: caret_byte,
                line,
                column,
            },
            visible_selection,
            visible_caret_position: CaretPosition {
                byte: visible_caret_byte,
                line: visible_line,
                column: visible_column,
            },
            display_map,
            blocks,
            outline,
            view_mode: self.view_mode,
            find_matches: Vec::new(),
            active_find_index: None,
        }
    }

    pub fn document_path(&self) -> Option<&PathBuf> {
        self.sync.path.as_ref()
    }

    pub fn current_document_dir(&self) -> Option<PathBuf> {
        self.sync.current_dir()
    }

    pub fn view_mode(&self) -> EditorViewMode {
        self.view_mode
    }

    pub fn set_view_mode(&mut self, view_mode: EditorViewMode) -> EditorEffects {
        if self.view_mode == view_mode {
            return EditorEffects::default();
        }

        self.view_mode = view_mode;
        *self.prev_display_map.borrow_mut() = None;
        EditorEffects {
            changed: false,
            selection_changed: true,
            reload_path: None,
        }
    }

    pub fn toggle_view_mode(&mut self) -> EditorEffects {
        let next = match self.view_mode {
            EditorViewMode::LivePreview => EditorViewMode::Source,
            EditorViewMode::Source => EditorViewMode::LivePreview,
        };
        self.set_view_mode(next)
    }

    pub(crate) fn toggle_task_range(&mut self, range: Range<usize>) -> EditorEffects {
        let current = self.document.text_for_range(range.clone());
        let replacement = if current.contains("[ ]") {
            current.replacen("[ ]", "[x]", 1)
        } else if current.contains("[x]") {
            current.replacen("[x]", "[ ]", 1)
        } else if current.contains("[X]") {
            current.replacen("[X]", "[ ]", 1)
        } else {
            return EditorEffects::default();
        };

        self.apply_edit(
            range,
            replacement,
            self.selection.clone(),
            "Updated task state",
        )
    }

    pub fn select_block_start(&mut self, block_id: u64) -> EditorEffects {
        let Some(block) = self.document.block_by_id(block_id) else {
            return EditorEffects::default();
        };

        self.update_selection(SelectionState::collapsed(block.content_range.start))
    }

    /// Move the collapsed cursor to `byte_offset` in the source document,
    /// clamping to the document length. Used by find-navigation in the app layer.
    pub fn select_source_offset(&mut self, byte_offset: usize) -> EditorEffects {
        let clamped = byte_offset.min(self.document.text().len());
        self.update_selection(SelectionState::collapsed(clamped))
    }

    pub fn replace_source_range(
        &mut self,
        range: Range<usize>,
        replacement: String,
    ) -> EditorEffects {
        let len = self.document.text().len();
        let clamped_start = range.start.min(len);
        let clamped_end = range.end.min(len);
        let cursor_after = clamped_start + replacement.len();
        let selection_after = SelectionState::collapsed(cursor_after);
        self.apply_edit(
            clamped_start..clamped_end,
            replacement,
            selection_after,
            "Replaced",
        )
    }

    pub fn open_path(&mut self, path: PathBuf) -> Result<EditorEffects> {
        let source = DocumentSource::from_disk(path.clone())?;
        self.replace_source(source);
        self.status_message = format!("Opened {}", path.display());
        Ok(EditorEffects {
            changed: true,
            selection_changed: true,
            reload_path: None,
        })
    }

    pub fn new_untitled(&mut self, suggested_path: Option<PathBuf>) -> EditorEffects {
        self.replace_source(DocumentSource::Empty {
            suggested_path: suggested_path.clone(),
        });
        self.status_message = "New file".to_string();
        EditorEffects {
            changed: true,
            selection_changed: true,
            reload_path: None,
        }
    }

    pub fn save(&mut self) -> Result<EditorEffects> {
        let path = self
            .sync
            .path
            .clone()
            .or_else(|| self.sync.suggested_path.clone())
            .context("cannot save without a target path")?;
        self.sync.saving = true;
        let text = self.document.text();
        if let Err(err) = fs::write(&path, text.as_bytes())
            .with_context(|| format!("failed to write {}", path.display()))
        {
            self.sync.saving = false;
            return Err(err);
        }

        let modified_at = file_modified_at(&path);
        self.sync.mark_saved(path.clone(), text, modified_at);
        self.status_message = format!("Saved {}", path.display());
        Ok(EditorEffects {
            changed: true,
            selection_changed: false,
            reload_path: None,
        })
    }

    pub fn save_as(&mut self, path: PathBuf) -> Result<EditorEffects> {
        self.sync.set_path(path.clone());
        self.save()
    }

    pub fn dispatch(&mut self, command: EditCommand) -> EditorEffects {
        match command {
            EditCommand::SyncDocumentState { text, selection } => {
                self.sync_document_state(text, selection)
            }
            EditCommand::SetSelection { selection } => self.update_selection(selection),
            EditCommand::ReplaceSelection { text } => {
                self.replace_selection_with_text(text, None, "Edited document")
            }
            EditCommand::InsertBreak { plain } => self.insert_break(plain),
            EditCommand::ToggleInlineMarkup { before, after } => {
                self.toggle_inline_markup(before, after)
            }
            EditCommand::InsertLink => self.insert_link(),
            EditCommand::InsertImage => self.insert_image_placeholder(),
            EditCommand::Indent => self.adjust_current_block(true),
            EditCommand::Outdent => self.adjust_current_block(false),
            EditCommand::ToggleBlockquote => self.toggle_blockquote(),
            EditCommand::ToggleBulletList => self.toggle_list(false),
            EditCommand::ToggleOrderedList => self.toggle_list(true),
            EditCommand::ToggleTaskList => self.set_task_list_markup(),
            EditCommand::InsertHorizontalRule => self.insert_horizontal_rule(),
            EditCommand::InsertCodeFence => self.insert_code_fence(),
            EditCommand::InsertMermaidDiagram => self.insert_mermaid_diagram(),
            EditCommand::InsertTable => self.insert_table(),
            EditCommand::InsertInlineMath => self.insert_inline_math(),
            EditCommand::InsertMathBlock => self.insert_math_block(),
            EditCommand::InsertHtmlBlock => self.insert_html_block(),
            EditCommand::InsertCallout => self.insert_callout(),
            EditCommand::InsertToc => self.insert_toc(),
            EditCommand::InsertFootnote => self.insert_footnote(),
            EditCommand::InsertFrontMatter => self.insert_front_matter(),
            EditCommand::MoveCaret {
                direction,
                preferred_column,
            } => self.move_caret_to_adjacent_block(direction, preferred_column),
            EditCommand::DeleteBackward => self.delete_backward(),
            EditCommand::DeleteForward => self.delete_forward(),
            EditCommand::DeleteSurroundingPair {
                before_len,
                after_len,
            } => self.delete_surrounding_pair(before_len, after_len),
            EditCommand::ToggleHeading { depth } => self.toggle_heading(depth),
            EditCommand::Undo => self.undo(),
            EditCommand::Redo => self.redo(),
            EditCommand::ReloadConflict => self.reload_conflict_from_disk(),
            EditCommand::KeepCurrentConflict => self.keep_current_conflicted_version(),
        }
    }

    pub fn apply_file_event(&mut self, event: FileSyncEvent) -> EditorEffects {
        match event {
            FileSyncEvent::Changed(path) => {
                let modified_at = file_modified_at(&path);
                if self.sync.path.as_ref() == Some(&path)
                    && !self.sync.has_same_disk_timestamp(modified_at)
                {
                    EditorEffects {
                        changed: false,
                        selection_changed: false,
                        reload_path: Some(path),
                    }
                } else {
                    EditorEffects::default()
                }
            }
            FileSyncEvent::Removed(path) => {
                if self.sync.path.as_ref() == Some(&path) {
                    self.sync.mark_missing();
                    self.status_message = format!("File removed: {}", path.display());
                    EditorEffects {
                        changed: true,
                        selection_changed: false,
                        reload_path: None,
                    }
                } else {
                    EditorEffects::default()
                }
            }
            FileSyncEvent::Relocated { from, to } => {
                if self.sync.path.as_ref() == Some(&from) {
                    self.sync.relocate(to.clone());
                    self.status_message = format!("File moved to {}", to.display());
                    EditorEffects {
                        changed: true,
                        selection_changed: false,
                        reload_path: Some(to),
                    }
                } else {
                    EditorEffects::default()
                }
            }
            FileSyncEvent::Unknown => EditorEffects::default(),
        }
    }

    pub fn apply_disk_state(
        &mut self,
        path: PathBuf,
        disk_text: String,
        modified_at: Option<SystemTime>,
    ) -> EditorEffects {
        if self.sync.path.as_ref() != Some(&path) {
            return EditorEffects::default();
        }

        if self.sync.has_same_disk_timestamp(modified_at) {
            return EditorEffects::default();
        }

        let current_text = self.document.text();
        if self.sync.dirty && current_text != disk_text {
            self.sync.mark_conflict(disk_text, modified_at);
            self.status_message = "External changes detected".to_string();
            return EditorEffects {
                changed: true,
                selection_changed: false,
                reload_path: None,
            };
        }

        if current_text == disk_text {
            self.sync.modified_at = modified_at;
            self.sync.missing_on_disk = false;
            return EditorEffects::default();
        }

        self.replace_document_from_text(disk_text.clone(), SelectionState::collapsed(0));
        self.sync
            .mark_loaded_from_disk(path.clone(), disk_text, modified_at);
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.status_message = format!("Reloaded {}", path.display());

        EditorEffects {
            changed: true,
            selection_changed: true,
            reload_path: None,
        }
    }

    fn replace_source(&mut self, source: DocumentSource) {
        let view_mode = self.view_mode;
        *self = Self::new(source, self.sync_policy);
        self.view_mode = view_mode;
    }
}

impl EditorController {
    fn sync_document_state(&mut self, text: String, selection: SelectionState) -> EditorEffects {
        let selection = clamp_selection_to_text(&text, selection);
        let current_text = self.document.text();
        if current_text == text {
            return self.update_selection(selection);
        }

        let Some((range, replacement)) = compute_document_diff(&current_text, &text) else {
            return self.update_selection(selection);
        };

        self.apply_edit(range, replacement, selection, "Edited document")
    }

    fn update_selection(&mut self, selection: SelectionState) -> EditorEffects {
        let current_text = self.document.text();
        let selection = clamp_selection_to_text(&current_text, selection);
        if self.selection == selection {
            return EditorEffects::default();
        }

        self.selection = selection;
        EditorEffects {
            changed: false,
            selection_changed: true,
            reload_path: None,
        }
    }

    fn insert_break(&mut self, plain: bool) -> EditorEffects {
        let range = self.selection.range();
        let current_block = self.current_block().cloned();
        if let Some(block) = current_block.as_ref()
            && block.kind == BlockKind::Table
        {
            return self.navigate_table_from_block(block, TableNavDirection::Down);
        }

        if !plain
            && range.is_empty()
            && let Some(effect) = self.close_opening_code_fence_line(range.start)
        {
            return effect;
        }

        if !plain
            && range.is_empty()
            && let Some(block) = current_block.as_ref()
            && matches!(block.kind, BlockKind::CodeFence { .. })
            && range.start == block.byte_range.end
        {
            let block_span_text = self.document.block_span_text(block);
            if let Some(transform) = semantic_enter_transform(
                &block.kind,
                &block_span_text,
                None,
                block_span_text.len(),
            ) {
                let selection_after =
                    SelectionState::collapsed(block.byte_range.start + transform.cursor_offset);
                return self.apply_edit(
                    block.byte_range.clone(),
                    transform.replacement,
                    selection_after,
                    "Updated block structure",
                );
            }
        }

        if plain {
            let start = self.selection.range().start;
            let selection_after = SelectionState::collapsed(start + 1);
            return self.replace_selection_with_text(
                "\n".to_string(),
                Some(selection_after),
                "Inserted line break",
            );
        }

        if let Some(block) = &current_block
            && range.is_empty()
            && range.start >= block.content_range.start
            && range.end <= block.content_range.end
        {
            let block_text = self.document.block_text(block);
            let local_cursor = range.start.saturating_sub(block.content_range.start);
            if local_cursor == block_text.len() {
                if let Some(action) = detect_auto_format(&block.kind, &block_text) {
                    return self.apply_auto_format_on_enter(&action);
                }
            }
        }

        if selection_spans_multiple_blocks(&self.document, &range) {
            let selection_after = SelectionState::collapsed(range.start + 1);
            return self.replace_selection_with_text(
                "\n".to_string(),
                Some(selection_after),
                "Inserted line break",
            );
        }

        let Some(block) = current_block else {
            return EditorEffects::default();
        };
        let block_text = self.document.block_text(&block);
        if range.is_empty()
            && matches!(block.kind, BlockKind::Raw | BlockKind::Paragraph)
            && self.document.blocks().len() == 1
            && raw_block_is_only_whitespace(&block_text)
        {
            let selection_after = SelectionState::collapsed(range.start + 1);
            return self.replace_selection_with_text(
                "\n\n".to_string(),
                Some(selection_after),
                "Inserted paragraph break",
            );
        }
        if let Some(effect) = self.insert_break_in_eof_empty_paragraph(&block, &range) {
            return effect;
        }
        if range.start < block.content_range.start || range.end > block.content_range.end {
            let selection_after = SelectionState::collapsed(range.start + 1);
            return self.replace_selection_with_text(
                "\n".to_string(),
                Some(selection_after),
                "Inserted line break",
            );
        }
        if range.is_empty()
            && block_text.is_empty()
            && matches!(block.kind, BlockKind::Raw | BlockKind::Paragraph)
        {
            let selection_after = SelectionState::collapsed(range.start + 1);
            return self.replace_selection_with_text(
                "\n".to_string(),
                Some(selection_after),
                "Inserted line break",
            );
        }
        let local_cursor = self
            .selection
            .cursor()
            .saturating_sub(block.content_range.start);
        let local_selection = (!range.is_empty()).then_some(
            range.start.saturating_sub(block.content_range.start)
                ..range.end.saturating_sub(block.content_range.start),
        );
        if let Some(transform) = pipe_table_enter_transform(
            &block.kind,
            &block_text,
            local_selection.clone(),
            local_cursor,
        ) {
            let trailing = self.document.block_trailing_text(&block);
            let replacement = format!("{}{}", transform.replacement, trailing);
            let selection_after =
                SelectionState::collapsed(block.byte_range.start + transform.cursor_offset);
            return self.apply_edit(
                block.byte_range,
                replacement,
                selection_after,
                "Created table",
            );
        }
        let Some(transform) =
            semantic_enter_transform(&block.kind, &block_text, local_selection, local_cursor)
        else {
            let selection_after = SelectionState::collapsed(range.start + 1);
            return self.replace_selection_with_text(
                "\n".to_string(),
                Some(selection_after),
                "Inserted line break",
            );
        };

        let trailing = self.document.block_trailing_text(&block);
        let replacement = format!("{}{}", transform.replacement, trailing);
        let selection_after =
            SelectionState::collapsed(block.byte_range.start + transform.cursor_offset);
        self.apply_edit(
            block.byte_range,
            replacement,
            selection_after,
            "Updated block structure",
        )
    }

    fn apply_auto_format_on_enter(&mut self, action: &AutoFormatAction) -> EditorEffects {
        match action {
            AutoFormatAction::Heading { depth } => {
                let effects = self.toggle_heading(*depth);
                if effects.changed {
                    self.insert_break(false)
                } else {
                    effects
                }
            }
            AutoFormatAction::Blockquote => {
                let effects = self.toggle_blockquote();
                if effects.changed {
                    self.insert_break(false)
                } else {
                    effects
                }
            }
            AutoFormatAction::BulletList => {
                let effects = self.toggle_list(false);
                if effects.changed {
                    self.insert_break(false)
                } else {
                    effects
                }
            }
            AutoFormatAction::OrderedList => {
                let effects = self.toggle_list(true);
                if effects.changed {
                    self.insert_break(false)
                } else {
                    effects
                }
            }
            AutoFormatAction::TaskList => {
                let effects = self.set_task_list_markup();
                if effects.changed {
                    self.insert_break(false)
                } else {
                    effects
                }
            }
            AutoFormatAction::HorizontalRule => {
                let effects = self.insert_horizontal_rule();
                if effects.changed {
                    self.insert_break(false)
                } else {
                    effects
                }
            }
            AutoFormatAction::CodeFence { marker, info } => {
                self.insert_code_fence_template(marker, info)
            }
            AutoFormatAction::MathBlock => {
                let effects = self.insert_math_block();
                effects
            }
        }
    }

    fn toggle_inline_markup(&mut self, before: String, after: String) -> EditorEffects {
        let range = self.selection.range();
        let selected_text = self.document.text_for_range(range.clone());
        let (replacement, cursor) = if selected_text.is_empty() {
            let replacement = format!("{before}{after}");
            let cursor = range.start + before.len();
            (replacement, cursor)
        } else {
            let replacement = format!("{before}{selected_text}{after}");
            let cursor = range.start + replacement.len();
            (replacement, cursor)
        };
        self.apply_edit(
            range,
            replacement,
            SelectionState::collapsed(cursor),
            "Updated formatting",
        )
    }

    fn insert_link(&mut self) -> EditorEffects {
        let range = self.selection.range();
        let selected_text = self.document.text_for_range(range.clone());
        let url_placeholder = "https://";
        if is_url_like(&selected_text) {
            let label_placeholder = "text";
            let destination = Self::escape_link_destination(&selected_text);
            let replacement = format!("[{label_placeholder}]({destination})");
            let selection_after = SelectionState {
                anchor_byte: range.start + 1 + label_placeholder.len(),
                head_byte: range.start + 1,
                preferred_column: None,
                affinity: SelectionAffinity::Downstream,
            };
            return self.apply_edit(range, replacement, selection_after, "Inserted link");
        }
        let raw_label = if selected_text.is_empty() {
            "text"
        } else {
            selected_text.as_str()
        };
        let label = Self::escape_link_label(raw_label);
        let replacement = format!("[{label}]({url_placeholder})");
        let url_start = range.start + label.len() + 3;
        let selection_after = SelectionState {
            anchor_byte: url_start,
            head_byte: url_start + url_placeholder.len(),
            preferred_column: None,
            affinity: SelectionAffinity::Downstream,
        };
        self.apply_edit(range, replacement, selection_after, "Inserted link")
    }

    fn insert_image_placeholder(&mut self) -> EditorEffects {
        let range = self.selection.range();
        let selected_text = self.document.text_for_range(range.clone());
        let url_placeholder = "image.png";
        if is_url_like(&selected_text) || looks_like_image_path(&selected_text) {
            let alt_placeholder = "alt";
            let destination = Self::escape_link_destination(&selected_text);
            let replacement = format!("![{alt_placeholder}]({destination})");
            let selection_after = SelectionState {
                anchor_byte: range.start + 2 + alt_placeholder.len(),
                head_byte: range.start + 2,
                preferred_column: None,
                affinity: SelectionAffinity::Downstream,
            };
            return self.apply_edit(range, replacement, selection_after, "Inserted image");
        }
        let raw_alt = if selected_text.is_empty() {
            "alt"
        } else {
            selected_text.as_str()
        };
        let alt = Self::escape_link_label(raw_alt);
        let replacement = format!("![{alt}]({url_placeholder})");
        let (edit_range, replacement, base_offset) = if selected_text.is_empty()
            && let Some(block) = self.current_block().cloned()
            && self.document.block_text(&block).trim().is_empty()
        {
            (
                block.content_range.clone(),
                format!("{replacement}\n\n"),
                block.content_range.start,
            )
        } else {
            let base_offset = range.start;
            (range, replacement, base_offset)
        };
        let url_start = base_offset + alt.len() + 4;
        let selection_after = SelectionState {
            anchor_byte: url_start,
            head_byte: url_start + url_placeholder.len(),
            preferred_column: None,
            affinity: SelectionAffinity::Downstream,
        };
        self.apply_edit(edit_range, replacement, selection_after, "Inserted image")
    }

    fn insert_inline_math(&mut self) -> EditorEffects {
        let range = self.selection.range();
        let selected_text = self.document.text_for_range(range.clone());
        let body = if selected_text.is_empty() {
            "x".to_string()
        } else {
            selected_text
        };
        let replacement = format!("${body}$");
        let selection_after = if body == "x" {
            SelectionState {
                anchor_byte: range.start + replacement.len() - 1,
                head_byte: range.start + 1,
                preferred_column: None,
                affinity: SelectionAffinity::Downstream,
            }
        } else {
            SelectionState::collapsed(range.start + replacement.len())
        };
        self.apply_edit(range, replacement, selection_after, "Inserted inline math")
    }

    fn close_opening_code_fence_line(&mut self, cursor: usize) -> Option<EditorEffects> {
        let text = self.document.text();
        let cursor = clamp_to_char_boundary(&text, cursor);
        let (line_start, line_end) = line_bounds_in_text(&text, cursor);
        if cursor != line_end {
            return None;
        }
        let line = &text[line_start..line_end];
        let marker = opening_fence_marker(line.trim())?;
        let replacement = format!("{line}\n\n{marker}");
        let selection_after = SelectionState::collapsed(line_start + line.len() + 1);
        Some(self.apply_edit(
            line_start..line_end,
            replacement,
            selection_after,
            "Inserted code fence",
        ))
    }

    fn escape_link_label(text: &str) -> String {
        text.replace('\\', r"\\").replace(']', r"\]")
    }

    fn escape_link_destination(text: &str) -> String {
        text.replace('\\', r"\\").replace(')', r"\)")
    }

    fn adjust_current_block(&mut self, deepen: bool) -> EditorEffects {
        let Some(block) = self.current_block().cloned() else {
            return EditorEffects::default();
        };
        let current = self.document.block_text(&block);
        let relative_cursor = self
            .selection
            .cursor()
            .saturating_sub(block.content_range.start);
        let local_selection = self.selection.range().start.saturating_sub(block.content_range.start)
            ..self.selection.range().end.saturating_sub(block.content_range.start);
        if block.kind == BlockKind::List
            && !local_selection.is_empty()
            && let Some(transform) = adjust_selected_list_markup(&current, local_selection.clone(), deepen)
        {
            let selection_after = SelectionState {
                anchor_byte: block.content_range.start + transform.selection.start,
                head_byte: block.content_range.start + transform.selection.end,
                preferred_column: None,
                affinity: self.selection.affinity,
            };
            return self.apply_edit(
                block.content_range,
                transform.replacement,
                selection_after,
                "Adjusted selected list structure",
            );
        }
        if block.kind == BlockKind::List
            && let Some(transform) = adjust_list_markup_at_cursor(&current, relative_cursor, deepen)
        {
            let selection_after =
                SelectionState::collapsed(block.content_range.start + transform.cursor_offset);
            return self.apply_edit(
                block.content_range,
                transform.replacement,
                selection_after,
                "Adjusted list structure",
            );
        }
        if matches!(block.kind, BlockKind::Blockquote | BlockKind::Callout { .. })
            && !local_selection.is_empty()
            && let Some(transform) =
                adjust_selected_quoted_list_markup(&current, local_selection, deepen)
        {
            let selection_after = SelectionState {
                anchor_byte: block.content_range.start + transform.selection.start,
                head_byte: block.content_range.start + transform.selection.end,
                preferred_column: None,
                affinity: self.selection.affinity,
            };
            return self.apply_edit(
                block.content_range,
                transform.replacement,
                selection_after,
                "Adjusted selected quoted list structure",
            );
        }
        if matches!(block.kind, BlockKind::Blockquote | BlockKind::Callout { .. })
            && let Some(transform) =
                adjust_quoted_list_markup_at_cursor(&current, relative_cursor, deepen)
        {
            let selection_after =
                SelectionState::collapsed(block.content_range.start + transform.cursor_offset);
            return self.apply_edit(
                block.content_range,
                transform.replacement,
                selection_after,
                "Adjusted quoted list structure",
            );
        }

        let Some(updated) = adjust_block_markup(&current, deepen) else {
            return EditorEffects::default();
        };
        let new_cursor = block.content_range.start + cmp::min(relative_cursor, updated.len());
        self.apply_edit(
            block.content_range,
            updated,
            SelectionState::collapsed(new_cursor),
            "Adjusted block structure",
        )
    }

    fn toggle_heading(&mut self, depth: u8) -> EditorEffects {
        let Some(block) = self.current_block().cloned() else {
            return EditorEffects::default();
        };
        let current = self.document.block_text(&block);
        let target_depth = match block.kind {
            BlockKind::Heading {
                depth: current_depth,
            } if current_depth == depth && depth > 0 => 0,
            _ => depth.min(6),
        };
        let updated = set_heading_markup(&current, target_depth);
        let relative_cursor = self
            .selection
            .cursor()
            .saturating_sub(block.content_range.start);
        let new_cursor = block.content_range.start
            + remap_heading_cursor_after_markup_change(&current, &updated, relative_cursor);
        self.apply_edit(
            block.content_range,
            updated,
            SelectionState::collapsed(new_cursor),
            if target_depth == 0 {
                "Converted heading to paragraph"
            } else {
                "Updated heading level"
            },
        )
    }

    fn toggle_blockquote(&mut self) -> EditorEffects {
        let Some(block) = self.current_block().cloned() else {
            return EditorEffects::default();
        };
        let current = self.document.block_text(&block);
        let enabled = !matches!(block.kind, BlockKind::Blockquote);
        let updated = set_blockquote_markup(&current, enabled);
        let relative_cursor = self
            .selection
            .cursor()
            .saturating_sub(block.content_range.start);
        let new_cursor = block.content_range.start
            + remap_list_cursor_after_markup_change(&current, &updated, relative_cursor);
        self.apply_edit(
            block.content_range,
            updated,
            SelectionState::collapsed(new_cursor),
            if enabled {
                "Converted paragraph to blockquote"
            } else {
                "Converted blockquote to paragraph"
            },
        )
    }

    fn toggle_list(&mut self, ordered: bool) -> EditorEffects {
        let Some(block) = self.current_block().cloned() else {
            return EditorEffects::default();
        };
        let current = self.document.block_text(&block);
        let updated = set_list_markup(&current, ordered);
        let relative_cursor = self
            .selection
            .cursor()
            .saturating_sub(block.content_range.start);
        let new_cursor = block.content_range.start
            + remap_list_cursor_after_markup_change(&current, &updated, relative_cursor);
        self.apply_edit(
            block.content_range,
            updated,
            SelectionState::collapsed(new_cursor),
            if ordered {
                "Toggled ordered list"
            } else {
                "Toggled bullet list"
            },
        )
    }

    fn set_task_list_markup(&mut self) -> EditorEffects {
        let Some(block) = self.current_block().cloned() else {
            return EditorEffects::default();
        };
        let current = self.document.block_text(&block);
        let updated = set_task_list_markup(&current);
        let relative_cursor = self
            .selection
            .cursor()
            .saturating_sub(block.content_range.start);
        let new_cursor = block.content_range.start
            + remap_list_cursor_after_markup_change(&current, &updated, relative_cursor);
        self.apply_edit(
            block.content_range,
            updated,
            SelectionState::collapsed(new_cursor),
            "Converted paragraph to task list",
        )
    }

    fn insert_horizontal_rule(&mut self) -> EditorEffects {
        let Some(block) = self.current_block().cloned() else {
            return EditorEffects::default();
        };
        let current_text = self.document.block_text(&block);
        let is_empty = current_text.trim().is_empty();
        if is_empty {
            // Replace the empty paragraph with the rule and a following paragraph.
            let cursor = block.content_range.start + 5;
            self.apply_edit(
                block.content_range,
                "---\n\n".to_string(),
                SelectionState::collapsed(cursor),
                "Inserted horizontal rule",
            )
        } else {
            // Insert `\n\n---\n\n` after the current block's full byte range and
            // leave cursor in the trailing empty paragraph that follows the rule.
            let insert_pos = block.byte_range.end;
            let insertion = "\n\n---\n\n".to_string();
            // Cursor should land after the two-newline separator that follows `---`.
            // That puts it at: insert_pos + "\n\n---\n\n".len() - 1 … but since the
            // document will re-parse, we just place cursor at insert_pos + 6 which is
            // the start of the blank paragraph after `---`.
            let cursor = insert_pos + 6;
            self.apply_edit(
                insert_pos..insert_pos,
                insertion,
                SelectionState::collapsed(cursor),
                "Inserted horizontal rule",
            )
        }
    }

    fn insert_code_fence(&mut self) -> EditorEffects {
        self.insert_code_fence_template("```", "")
    }

    fn insert_code_fence_template(&mut self, marker: &str, info: &str) -> EditorEffects {
        let range = self.selection.range();
        let selected_text = self.document.text_for_range(range.clone());
        if !selected_text.is_empty() {
            let opening = if info.is_empty() {
                marker.to_string()
            } else {
                format!("{marker}{info}")
            };
            let body = selected_text.trim_matches('\n');
            let replacement = format!("{opening}\n{body}\n{marker}");
            let body_start = range.start + opening.len() + 1;
            let body_end = body_start + body.len();
            return self.apply_edit(
                range,
                replacement,
                SelectionState {
                    anchor_byte: body_end,
                    head_byte: body_start,
                    preferred_column: None,
                    affinity: SelectionAffinity::Downstream,
                },
                "Inserted code fence",
            );
        }

        let Some(block) = self.current_block().cloned() else {
            return EditorEffects::default();
        };
        let current_text = self.document.block_text(&block);
        let is_empty = current_text.trim().is_empty();
        let opening = if info.is_empty() {
            marker.to_string()
        } else {
            format!("{marker}{info}")
        };
        let template = format!("{opening}\n\n{marker}");
        let cursor_in_template = opening.len() + 1;
        if is_empty {
            let cursor = block.content_range.start + cursor_in_template;
            self.apply_edit(
                block.content_range,
                template,
                SelectionState::collapsed(cursor),
                "Inserted code fence",
            )
        } else {
            let insert_pos = block.byte_range.end;
            let insertion = format!("\n\n{template}");
            let cursor = insert_pos + 2 + cursor_in_template;
            self.apply_edit(
                insert_pos..insert_pos,
                insertion,
                SelectionState::collapsed(cursor),
                "Inserted code fence",
            )
        }
    }

    fn insert_mermaid_diagram(&mut self) -> EditorEffects {
        let range = self.selection.range();
        let selected_text = self.document.text_for_range(range.clone());
        if !selected_text.is_empty() {
            let document_text = self.document.text();
            let body = selected_text.trim_matches('\n');
            let prefix = if range.start == 0 || document_text[..range.start].ends_with("\n\n") {
                ""
            } else if document_text[..range.start].ends_with('\n') {
                "\n"
            } else {
                "\n\n"
            };
            let suffix = if range.end == document_text.len()
                || document_text[range.end..].starts_with("\n\n")
            {
                ""
            } else if document_text[range.end..].starts_with('\n') {
                "\n"
            } else {
                "\n\n"
            };
            let opening = "```mermaid\n";
            let replacement = format!("{prefix}{opening}{body}\n```{suffix}");
            let body_start = range.start + prefix.len() + opening.len();
            let body_end = body_start + body.len();
            return self.apply_edit(
                range,
                replacement,
                SelectionState {
                    anchor_byte: body_end,
                    head_byte: body_start,
                    preferred_column: None,
                    affinity: SelectionAffinity::Downstream,
                },
                "Inserted Mermaid diagram",
            );
        }

        let Some(block) = self.current_block().cloned() else {
            return EditorEffects::default();
        };
        let current_text = self.document.block_text(&block);
        let is_empty = current_text.trim().is_empty();
        let body = "graph TD\n  A[Start] --> B[End]";
        let template = format!("```mermaid\n{body}\n```\n\n");
        let body_start = "```mermaid\n".len();
        let body_end = body_start + body.len();
        let selection_for = |base: usize| SelectionState {
            anchor_byte: base + body_end,
            head_byte: base + body_start,
            preferred_column: None,
            affinity: SelectionAffinity::Downstream,
        };

        if is_empty {
            let base = block.content_range.start;
            self.apply_edit(
                block.content_range,
                template,
                selection_for(base),
                "Inserted Mermaid diagram",
            )
        } else {
            let insert_pos = block.byte_range.end;
            let insertion = format!("\n\n{template}");
            self.apply_edit(
                insert_pos..insert_pos,
                insertion,
                selection_for(insert_pos + 2),
                "Inserted Mermaid diagram",
            )
        }
    }

    fn insert_math_block(&mut self) -> EditorEffects {
        let range = self.selection.range();
        let selected_text = self.document.text_for_range(range.clone());
        if !selected_text.is_empty() {
            let document_text = self.document.text();
            let body = selected_text.trim_matches('\n');
            let prefix = if range.start == 0 || document_text[..range.start].ends_with("\n\n") {
                ""
            } else if document_text[..range.start].ends_with('\n') {
                "\n"
            } else {
                "\n\n"
            };
            let suffix = if range.end == document_text.len()
                || document_text[range.end..].starts_with("\n\n")
            {
                ""
            } else if document_text[range.end..].starts_with('\n') {
                "\n"
            } else {
                "\n\n"
            };
            let replacement = format!("{prefix}$$\n{body}\n$${suffix}");
            let body_start = range.start + prefix.len() + 3;
            let body_end = body_start + body.len();
            return self.apply_edit(
                range,
                replacement,
                SelectionState {
                    anchor_byte: body_end,
                    head_byte: body_start,
                    preferred_column: None,
                    affinity: SelectionAffinity::Downstream,
                },
                "Inserted math block",
            );
        }

        let Some(block) = self.current_block().cloned() else {
            return EditorEffects::default();
        };
        let current_text = self.document.block_text(&block);
        let is_empty = current_text.trim().is_empty();
        let template = "$$\n$$";
        if is_empty {
            let cursor = block.content_range.start + 3;
            self.apply_edit(
                block.content_range,
                template.to_string(),
                SelectionState::collapsed(cursor),
                "Inserted math block",
            )
        } else {
            let insert_pos = block.byte_range.end;
            let insertion = format!("\n{template}");
            let cursor = insert_pos + 1 + 3;
            self.apply_edit(
                insert_pos..insert_pos,
                insertion,
                SelectionState::collapsed(cursor),
                "Inserted math block",
            )
        }
    }

    fn insert_html_block(&mut self) -> EditorEffects {
        let range = self.selection.range();
        let selected_text = self.document.text_for_range(range.clone());
        if !selected_text.is_empty() {
            let document_text = self.document.text();
            let body = selected_text.trim_matches('\n');
            let prefix = if range.start == 0 || document_text[..range.start].ends_with("\n\n") {
                ""
            } else if document_text[..range.start].ends_with('\n') {
                "\n"
            } else {
                "\n\n"
            };
            let suffix = if range.end == document_text.len()
                || document_text[range.end..].starts_with("\n\n")
            {
                ""
            } else if document_text[range.end..].starts_with('\n') {
                "\n"
            } else {
                "\n\n"
            };
            let opening = "<div>\n";
            let indent = "  ";
            let closing = "\n</div>";
            let replacement = format!("{prefix}{opening}{indent}{body}{closing}{suffix}");
            let body_start = range.start + prefix.len() + opening.len() + indent.len();
            let body_end = body_start + body.len();
            return self.apply_edit(
                range,
                replacement,
                SelectionState {
                    anchor_byte: body_end,
                    head_byte: body_start,
                    preferred_column: None,
                    affinity: SelectionAffinity::Downstream,
                },
                "Inserted HTML block",
            );
        }

        let Some(block) = self.current_block().cloned() else {
            return EditorEffects::default();
        };
        let current_text = self.document.block_text(&block);
        let is_empty = current_text.trim().is_empty();
        let template = "<div>\n  Content\n</div>\n\n";
        let selection_start = "<div>\n  ".len();
        let selection_end = selection_start + "Content".len();

        if is_empty {
            let base_offset = block.content_range.start;
            self.apply_edit(
                block.content_range,
                template.to_string(),
                SelectionState {
                    anchor_byte: base_offset + selection_end,
                    head_byte: base_offset + selection_start,
                    preferred_column: None,
                    affinity: SelectionAffinity::Downstream,
                },
                "Inserted HTML block",
            )
        } else {
            let insert_pos = block.byte_range.end;
            let insertion = format!("\n\n{template}");
            let base_offset = insert_pos + 2;
            self.apply_edit(
                insert_pos..insert_pos,
                insertion,
                SelectionState {
                    anchor_byte: base_offset + selection_end,
                    head_byte: base_offset + selection_start,
                    preferred_column: None,
                    affinity: SelectionAffinity::Downstream,
                },
                "Inserted HTML block",
            )
        }
    }

    fn insert_callout(&mut self) -> EditorEffects {
        let range = self.selection.range();
        let selected_text = self.document.text_for_range(range.clone());
        if !selected_text.is_empty() {
            let document_text = self.document.text();
            let body = selected_text.trim_matches('\n');
            let prefix = if range.start == 0 || document_text[..range.start].ends_with("\n\n") {
                ""
            } else if document_text[..range.start].ends_with('\n') {
                "\n"
            } else {
                "\n\n"
            };
            let suffix = if range.end == document_text.len()
                || document_text[range.end..].starts_with("\n\n")
            {
                ""
            } else if document_text[range.end..].starts_with('\n') {
                "\n"
            } else {
                "\n\n"
            };
            let quoted_body = body
                .lines()
                .map(|line| format!("> {line}"))
                .collect::<Vec<_>>()
                .join("\n");
            let header = "> [!NOTE] Title\n";
            let replacement = format!("{prefix}{header}{quoted_body}{suffix}");
            let body_start = range.start + prefix.len() + header.len() + 2;
            let body_end = range.start + prefix.len() + header.len() + quoted_body.len();
            return self.apply_edit(
                range,
                replacement,
                SelectionState {
                    anchor_byte: body_end,
                    head_byte: body_start,
                    preferred_column: None,
                    affinity: SelectionAffinity::Downstream,
                },
                "Inserted callout",
            );
        }

        let Some(block) = self.current_block().cloned() else {
            return EditorEffects::default();
        };
        let current_text = self.document.block_text(&block);
        let is_empty = current_text.trim().is_empty();
        let template = "> [!NOTE] Title\n> ";
        let body_cursor = template.len();
        if is_empty {
            let base_offset = block.content_range.start;
            self.apply_edit(
                block.content_range,
                template.to_string(),
                SelectionState::collapsed(base_offset + body_cursor),
                "Inserted callout",
            )
        } else {
            let insert_pos = block.byte_range.end;
            let insertion = format!("\n\n{template}");
            self.apply_edit(
                insert_pos..insert_pos,
                insertion,
                SelectionState::collapsed(insert_pos + 2 + body_cursor),
                "Inserted callout",
            )
        }
    }

    fn insert_toc(&mut self) -> EditorEffects {
        let Some(block) = self.current_block().cloned() else {
            return EditorEffects::default();
        };
        let current_text = self.document.block_text(&block);
        let is_empty = current_text.trim().is_empty();
        let template = "[toc]";
        let replacement = format!("{template}\n\n");
        if is_empty {
            let base_offset = block.content_range.start;
            self.apply_edit(
                block.content_range,
                replacement,
                SelectionState::collapsed(base_offset + template.len() + 2),
                "Inserted table of contents",
            )
        } else {
            let insert_pos = block.byte_range.end;
            let insertion = format!("\n\n{replacement}");
            self.apply_edit(
                insert_pos..insert_pos,
                insertion,
                SelectionState::collapsed(insert_pos + 2 + template.len() + 2),
                "Inserted table of contents",
            )
        }
    }

    fn insert_footnote(&mut self) -> EditorEffects {
        let text = self.document.text();
        let label = next_footnote_label(&text);
        let reference = format!("[^{label}]");
        let definition_prefix = format!("[^{label}]: ");
        let range = self.selection.range();
        let selected_text = self.document.text_for_range(range.clone());
        let definition_text = if selected_text.is_empty() {
            "Footnote text"
        } else {
            selected_text.as_str()
        };
        let tail = &text[range.end..];
        let body_end = range.start + reference.len() + tail.len();
        let body_text = format!("{}{}{}", &text[..range.start], reference, tail);
        let separator = if body_text.is_empty() {
            ""
        } else if body_text.ends_with("\n\n") {
            ""
        } else if body_text.ends_with('\n') {
            "\n"
        } else {
            "\n\n"
        };

        let insertion = format!(
            "{reference}{tail}{separator}{definition_prefix}{definition_text}\n\n"
        );
        let definition_start = body_end + separator.len() + definition_prefix.len();
        let definition_end = definition_start + definition_text.len();

        self.apply_edit(
            range.start..text.len(),
            insertion,
            SelectionState {
                anchor_byte: definition_end,
                head_byte: definition_start,
                preferred_column: None,
                affinity: SelectionAffinity::Downstream,
            },
            "Inserted footnote",
        )
    }

    fn insert_front_matter(&mut self) -> EditorEffects {
        let text = self.document.text();
        let range = self.selection.range();
        let selected_text = self.document.text_for_range(range);
        let title = selected_text.trim();
        let title = if title.is_empty() { "Untitled" } else { title };
        let template = format!("---\ntitle: {title}\ndate: \ntags: []\n---\n\n");
        let title_start = "---\ntitle: ".len();
        let title_end = title_start + title.len();

        if text.trim().is_empty() {
            return self.apply_edit(
                0..text.len(),
                template,
                SelectionState {
                    anchor_byte: title_end,
                    head_byte: title_start,
                    preferred_column: None,
                    affinity: SelectionAffinity::Downstream,
                },
                "Inserted front matter",
            );
        }

        if text.starts_with("---\n") || text.starts_with("+++\n") {
            return self.update_selection(SelectionState::collapsed(0));
        }

        self.apply_edit(
            0..0,
            template,
            SelectionState {
                anchor_byte: title_end,
                head_byte: title_start,
                preferred_column: None,
                affinity: SelectionAffinity::Downstream,
            },
            "Inserted front matter",
        )
    }

    fn insert_table(&mut self) -> EditorEffects {
        let Some(block) = self.current_block().cloned() else {
            return EditorEffects::default();
        };
        let table_template = "| Column 1 | Column 2 | Column 3 |\n| --- | --- | --- |\n|  |  |  |";
        let current_text = self.document.block_text(&block);
        let is_empty = current_text.trim().is_empty();
        let (edit_range, insertion, base_offset) = if is_empty {
            (
                block.content_range.clone(),
                table_template.to_string(),
                block.content_range.start,
            )
        } else {
            let insert_pos = block.byte_range.end;
            (
                insert_pos..insert_pos,
                format!("\n\n{table_template}"),
                insert_pos + 2,
            )
        };
        let table = TableModel::parse(table_template);
        let cursor = table
            .cell_source_range(TableCellRef {
                visible_row: 1,
                column: 0,
            })
            .map(|range| base_offset + range.start)
            .unwrap_or(base_offset + table_template.len());
        self.apply_edit(
            edit_range,
            insertion,
            SelectionState::collapsed(cursor),
            "Inserted table",
        )
    }

    fn move_caret_to_adjacent_block(
        &mut self,
        direction: isize,
        preferred_column: Option<usize>,
    ) -> EditorEffects {
        if self.document.blocks().is_empty() {
            return EditorEffects::default();
        }

        let current = self.document.block_index_at_offset(self.selection.cursor());
        let next = if direction >= 0 {
            cmp::min(current + 1, self.document.blocks().len().saturating_sub(1))
        } else {
            current.saturating_sub(1)
        };
        if next == current {
            return EditorEffects::default();
        }

        let column = preferred_column
            .or(self.selection.preferred_column)
            .unwrap_or_else(|| self.snapshot().caret_position.column);
        let Some(block) = self.document.blocks().get(next) else {
            return EditorEffects::default();
        };
        let text = self.document.block_text(block);
        let local_cursor = boundary_cursor_offset(&text, direction, column);
        let mut selection = SelectionState::collapsed(block.content_range.start + local_cursor);
        selection.preferred_column = Some(column);
        self.update_selection(selection)
    }

    fn navigate_table_from_block(
        &mut self,
        block: &BlockProjection,
        direction: TableNavDirection,
    ) -> EditorEffects {
        let table = TableModel::parse(&self.document.block_text(block));
        if table.is_empty() {
            return EditorEffects::default();
        }

        let local_cursor = self
            .selection
            .cursor()
            .saturating_sub(block.content_range.start);
        let current_cell = table
            .cell_ref_for_source_offset(local_cursor, self.selection.affinity)
            .or_else(|| table.first_cell());
        let Some(current_cell) = current_cell else {
            return EditorEffects::default();
        };

        if let Some(target_cell) = table.next_cell_ref(current_cell, direction) {
            return self.update_selection(table_cell_selection(block, &table, target_cell));
        }

        if direction == TableNavDirection::Backward {
            return EditorEffects::default();
        }

        let replacement = table.append_empty_row();
        let target_cell = TableCellRef {
            visible_row: table.visible_row_count(),
            column: match direction {
                TableNavDirection::Forward => 0,
                TableNavDirection::Backward => 0,
                TableNavDirection::Down => {
                    if current_cell.column + 1 == table.column_count() {
                        0
                    } else {
                        current_cell
                            .column
                            .min(table.column_count().saturating_sub(1))
                    }
                }
            },
        };
        let rebuilt = TableModel::parse(&replacement);
        let selection_after = table_cell_selection(block, &rebuilt, target_cell);
        let trailing = self.document.block_trailing_text(block);
        self.apply_edit(
            block.byte_range.clone(),
            format!("{}{}", replacement, trailing),
            selection_after,
            "Extended table",
        )
    }

    fn delete_backward(&mut self) -> EditorEffects {
        if !self.selection.is_collapsed() {
            let selection_after = SelectionState::collapsed(self.selection.range().start);
            return self.replace_selection_with_text(
                String::new(),
                Some(selection_after),
                "Deleted selection",
            );
        }

        if let Some(effect) = self.delete_backward_in_table() {
            return effect;
        }
        if let Some(effect) = self.delete_backward_eof_empty_paragraph() {
            return effect;
        }
        if let Some(effect) = self.delete_backward_collapsed_inter_block_gap() {
            return effect;
        }
        if let Some(effect) = self.delete_backward_structural() {
            return effect;
        }
        if let Some(effect) = self.delete_backward_empty_pair() {
            return effect;
        }

        let text = self.document.text();
        let cursor = self.selection.cursor();
        if cursor == 0 {
            return EditorEffects::default();
        }

        let start = previous_char_boundary(&text, cursor);
        self.apply_edit(
            start..cursor,
            String::new(),
            SelectionState::collapsed(start),
            "Deleted text",
        )
    }

    fn delete_forward(&mut self) -> EditorEffects {
        if !self.selection.is_collapsed() {
            let selection_after = SelectionState::collapsed(self.selection.range().start);
            return self.replace_selection_with_text(
                String::new(),
                Some(selection_after),
                "Deleted selection",
            );
        }

        if let Some(effect) = self.delete_forward_in_table() {
            return effect;
        }
        if let Some(effect) = self.delete_forward_eof_empty_paragraph() {
            return effect;
        }
        if let Some(effect) = self.delete_forward_collapsed_inter_block_gap() {
            return effect;
        }
        if let Some(effect) = self.delete_forward_structural() {
            return effect;
        }
        if let Some(effect) = self.delete_forward_empty_pair() {
            return effect;
        }

        let text = self.document.text();
        let cursor = self.selection.cursor();
        if cursor >= text.len() {
            return EditorEffects::default();
        }

        let end = next_char_boundary(&text, cursor);
        self.apply_edit(
            cursor..end,
            String::new(),
            SelectionState::collapsed(cursor),
            "Deleted text",
        )
    }

    fn delete_surrounding_pair(&mut self, before_len: usize, after_len: usize) -> EditorEffects {
        let cursor = self.selection.cursor();
        let start = cursor.saturating_sub(before_len);
        let end = cursor + after_len;
        if start >= end || end > self.document.text().len() {
            return EditorEffects::default();
        }
        self.apply_edit(
            start..end,
            String::new(),
            SelectionState::collapsed(start),
            "Deleted auto-pair",
        )
    }

    fn delete_backward_empty_pair(&mut self) -> Option<EditorEffects> {
        let text = self.document.text();
        let cursor = self.selection.cursor();
        if cursor == 0 || cursor >= text.len() {
            return None;
        }

        let before_start = previous_char_boundary(&text, cursor);
        let before = text[before_start..cursor].chars().next()?;
        let after_end = next_char_boundary(&text, cursor);
        let after = text[cursor..after_end].chars().next()?;

        if auto_pair_closer_for(before) != Some(after) {
            return None;
        }

        Some(self.delete_surrounding_pair(cursor - before_start, after_end - cursor))
    }

    fn delete_forward_empty_pair(&mut self) -> Option<EditorEffects> {
        let text = self.document.text();
        let cursor = self.selection.cursor();
        if cursor >= text.len() {
            return None;
        }

        let opener_end = next_char_boundary(&text, cursor);
        let opener = text[cursor..opener_end].chars().next()?;
        let closer_end = next_char_boundary(&text, opener_end);
        let closer = text[opener_end..closer_end].chars().next()?;

        if auto_pair_closer_for(opener) != Some(closer) {
            return None;
        }

        Some(self.delete_surrounding_pair(0, closer_end - cursor))
    }

    fn delete_backward_in_table(&mut self) -> Option<EditorEffects> {
        self.delete_in_table(true)
    }

    fn delete_forward_in_table(&mut self) -> Option<EditorEffects> {
        self.delete_in_table(false)
    }

    fn delete_in_table(&mut self, backwards: bool) -> Option<EditorEffects> {
        let block = self.current_block()?.clone();
        if block.kind != BlockKind::Table {
            return None;
        }

        let table = TableModel::parse(&self.document.block_text(&block));
        if table.is_empty() {
            return Some(consume_handled_action());
        }

        let local_cursor = self
            .selection
            .cursor()
            .saturating_sub(block.content_range.start);
        let current_cell = table
            .cell_ref_for_source_offset(local_cursor, self.selection.affinity)
            .or_else(|| table.first_cell())?;
        let cell_range = table.cell_source_range(current_cell)?;
        let cell_source = table.cell_source_text(current_cell).unwrap_or("");
        let cursor_in_cell =
            local_cursor.clamp(cell_range.start, cell_range.end) - cell_range.start;

        let (updated_cell_source, cursor_after) = if backwards {
            if cursor_in_cell == 0 {
                return Some(consume_handled_action());
            }

            let delete_start = previous_char_boundary(cell_source, cursor_in_cell);
            let mut updated =
                String::with_capacity(cell_source.len() - (cursor_in_cell - delete_start));
            updated.push_str(&cell_source[..delete_start]);
            updated.push_str(&cell_source[cursor_in_cell..]);
            (updated, delete_start)
        } else {
            if cursor_in_cell >= cell_source.len() {
                return Some(consume_handled_action());
            }

            let delete_end = next_char_boundary(cell_source, cursor_in_cell);
            let mut updated =
                String::with_capacity(cell_source.len() - (delete_end - cursor_in_cell));
            updated.push_str(&cell_source[..cursor_in_cell]);
            updated.push_str(&cell_source[delete_end..]);
            (updated, cursor_in_cell)
        };

        let replacement = table.rebuild_markdown_with_override(current_cell, updated_cell_source);
        let rebuilt = TableModel::parse(&replacement);
        let selection_after =
            table_cell_selection_with_offset(&block, &rebuilt, current_cell, cursor_after);
        let trailing = self.document.block_trailing_text(&block);
        Some(self.apply_edit(
            block.byte_range.clone(),
            format!("{}{}", replacement, trailing),
            selection_after,
            "Deleted text",
        ))
    }
}

impl EditorController {
    fn insert_break_in_eof_empty_paragraph(
        &mut self,
        block: &BlockProjection,
        range: &Range<usize>,
    ) -> Option<EditorEffects> {
        if !range.is_empty()
            || !is_eof_empty_paragraph_block(&self.document, block, self.selection.cursor())
        {
            return None;
        }

        let (replacement, status_message) = if block.byte_range.is_empty() {
            ("\n\n", "Inserted paragraph break")
        } else {
            ("\n", "Inserted line break")
        };
        let selection_after = SelectionState::collapsed(range.start + 1);
        Some(self.replace_selection_with_text(
            replacement.to_string(),
            Some(selection_after),
            status_message,
        ))
    }

    fn delete_backward_eof_empty_paragraph(&mut self) -> Option<EditorEffects> {
        let current = self.current_block()?.clone();
        if self.selection.cursor() != current.content_range.start
            || !is_eof_empty_paragraph_block(&self.document, &current, self.selection.cursor())
        {
            return None;
        }

        let current_index = self.document.block_index_by_id(current.id)?;
        if current_index == 0 {
            return None;
        }

        let previous = self.document.blocks().get(current_index - 1)?.clone();
        if !supports_eof_empty_paragraph_predecessor_kind(&previous.kind) {
            return None;
        }

        let deletion_end = current.byte_range.end.max(previous.byte_range.end);
        let deletion_range = previous.content_range.end..deletion_end;
        if deletion_range.is_empty() {
            return None;
        }

        Some(self.apply_edit(
            deletion_range,
            String::new(),
            SelectionState::collapsed(previous.content_range.end),
            "Deleted empty block",
        ))
    }

    fn delete_forward_eof_empty_paragraph(&mut self) -> Option<EditorEffects> {
        let current = self.current_block()?.clone();
        if self.selection.cursor() != current.byte_range.start
            || !is_eof_empty_paragraph_block(&self.document, &current, self.selection.cursor())
        {
            return None;
        }

        (current.byte_range.len() == 1).then(EditorEffects::default)
    }

    fn delete_backward_collapsed_inter_block_gap(&mut self) -> Option<EditorEffects> {
        let current = self.current_block()?.clone();
        if self.selection.cursor() != current.content_range.start {
            return None;
        }

        let gap = inter_block_collapsed_gap(&self.document, &current)?;
        Some(self.apply_edit(
            gap.replacement_range,
            "\n".to_string(),
            SelectionState::collapsed(gap.selection_after),
            "Deleted empty paragraph",
        ))
    }

    fn delete_forward_collapsed_inter_block_gap(&mut self) -> Option<EditorEffects> {
        let current = self.current_block()?.clone();
        if self.selection.cursor() != current.content_range.start {
            return None;
        }

        let gap = inter_block_collapsed_gap(&self.document, &current)?;
        Some(self.apply_edit(
            gap.replacement_range,
            "\n".to_string(),
            SelectionState::collapsed(gap.selection_after),
            "Deleted empty paragraph",
        ))
    }

    fn delete_forward_structural(&mut self) -> Option<EditorEffects> {
        let current = self.current_block()?.clone();
        if self.selection.cursor() != current.content_range.end {
            return None;
        }

        let current_index = self.document.block_index_by_id(current.id)?;
        let next = self.document.blocks().get(current_index + 1)?.clone();
        if !supports_boundary_backspace_target_kind(&current.kind)
            || !supports_boundary_backspace_target_kind(&next.kind)
        {
            return None;
        }

        let deletion_range = current.content_range.end..next.byte_range.start;
        if deletion_range.is_empty() {
            return None;
        }

        Some(self.apply_edit(
            deletion_range,
            String::new(),
            SelectionState::collapsed(current.content_range.end),
            "Deleted empty block",
        ))
    }

    fn delete_backward_structural(&mut self) -> Option<EditorEffects> {
        let current = self.current_block()?.clone();
        if self.selection.cursor() != current.content_range.start {
            return None;
        }

        let current_text = self.document.block_text(&current);
        if current_text.is_empty() && supports_empty_boundary_backspace_kind(&current.kind) {
            let current_index = self.document.block_index_by_id(current.id)?;
            if current_index == 0 {
                return None;
            }

            let previous = self.document.blocks().get(current_index - 1)?.clone();
            if !supports_boundary_backspace_target_kind(&previous.kind) {
                return None;
            }

            let deletion_range = if current_index + 1 < self.document.blocks().len()
                && !current.byte_range.is_empty()
            {
                current.byte_range.clone()
            } else {
                previous.content_range.end..previous.byte_range.end
            };
            if deletion_range.is_empty() {
                return None;
            }

            let selection_after = SelectionState::collapsed(previous.content_range.end);
            return Some(self.apply_edit(
                deletion_range,
                String::new(),
                selection_after,
                "Deleted empty block",
            ));
        }

        if matches!(
            current.kind,
            BlockKind::Heading { .. } | BlockKind::List | BlockKind::Blockquote
        ) {
            let updated = adjust_block_markup(&current_text, false)?;
            if updated == current_text {
                return None;
            }
            let content_range = current.content_range.clone();
            return Some(self.apply_edit(
                content_range.clone(),
                updated,
                SelectionState::collapsed(content_range.start),
                "Adjusted block structure",
            ));
        }

        None
    }

    fn replace_selection_with_text(
        &mut self,
        replacement: String,
        selection_after: Option<SelectionState>,
        status_message: &str,
    ) -> EditorEffects {
        let range = self.selection.range();
        let selection_after = selection_after
            .unwrap_or_else(|| SelectionState::collapsed(range.start + replacement.len()));
        self.apply_edit(range, replacement, selection_after, status_message)
    }

    fn undo(&mut self) -> EditorEffects {
        let Some(entry) = self.undo_stack.pop() else {
            return EditorEffects::default();
        };

        self.document.apply_transaction(Transaction::Replace {
            range: entry.after_range.clone(),
            replacement: entry.before_text.clone(),
        });
        self.selection =
            clamp_selection_to_text(&self.document.text(), entry.selection_before.clone());
        self.sync.mark_document_changed(&self.document.text());
        self.redo_stack.push(entry);
        self.status_message = "Undo".to_string();

        EditorEffects {
            changed: true,
            selection_changed: true,
            reload_path: None,
        }
    }

    fn redo(&mut self) -> EditorEffects {
        let Some(entry) = self.redo_stack.pop() else {
            return EditorEffects::default();
        };

        self.document.apply_transaction(Transaction::Replace {
            range: entry.before_range.clone(),
            replacement: entry.after_text.clone(),
        });
        self.selection =
            clamp_selection_to_text(&self.document.text(), entry.selection_after.clone());
        self.sync.mark_document_changed(&self.document.text());
        self.undo_stack.push(entry);
        self.status_message = "Redo".to_string();

        EditorEffects {
            changed: true,
            selection_changed: true,
            reload_path: None,
        }
    }

    fn reload_conflict_from_disk(&mut self) -> EditorEffects {
        let Some(path) = self.sync.path.clone() else {
            return EditorEffects::default();
        };

        let disk_text = match &self.sync.conflict {
            ConflictState::Conflict { disk_text, .. } => disk_text.clone(),
            ConflictState::Clean => return EditorEffects::default(),
        };
        let modified_at = file_modified_at(&path);

        self.replace_document_from_text(disk_text.clone(), SelectionState::collapsed(0));
        self.sync
            .mark_loaded_from_disk(path, disk_text, modified_at);
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.status_message = "Reloaded disk version".to_string();

        EditorEffects {
            changed: true,
            selection_changed: true,
            reload_path: None,
        }
    }

    fn keep_current_conflicted_version(&mut self) -> EditorEffects {
        if !matches!(self.sync.conflict, ConflictState::Conflict { .. }) {
            return EditorEffects::default();
        }

        self.sync.keep_current_conflicted_version();
        self.status_message = "Keeping current changes".to_string();
        EditorEffects {
            changed: true,
            selection_changed: false,
            reload_path: None,
        }
    }

    fn apply_edit(
        &mut self,
        range: Range<usize>,
        replacement: String,
        selection_after: SelectionState,
        status_message: &str,
    ) -> EditorEffects {
        let selection_before = self.selection.clone();
        let applied = self
            .document
            .apply_transaction(Transaction::Replace { range, replacement });
        let selection_after = clamp_selection_to_text(&self.document.text(), selection_after);

        self.undo_stack.push(EditHistoryEntry {
            before_range: applied.before_range.clone(),
            before_text: applied.before_text,
            after_range: applied.after_range,
            after_text: applied.after_text,
            selection_before,
            selection_after: selection_after.clone(),
        });
        self.redo_stack.clear();
        self.selection = selection_after;
        self.sync.mark_document_changed(&self.document.text());
        self.status_message = status_message.to_string();

        EditorEffects {
            changed: true,
            selection_changed: true,
            reload_path: None,
        }
    }

    fn replace_document_from_text(&mut self, text: String, selection: SelectionState) {
        self.document = DocumentBuffer::from_text(text.clone());
        self.selection = clamp_selection_to_text(&text, selection);
    }

    fn current_block(&self) -> Option<&BlockProjection> {
        self.document
            .blocks()
            .get(self.document.block_index_at_offset(self.selection.cursor()))
    }
}

fn next_footnote_label(text: &str) -> usize {
    let mut max_label = 0usize;
    let bytes = text.as_bytes();
    let mut index = 0usize;

    while let Some(relative) = text[index..].find("[^") {
        let start = index + relative + 2;
        let mut end = start;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
        if end > start && bytes.get(end) == Some(&b']') {
            if let Ok(label) = text[start..end].parse::<usize>() {
                max_label = max_label.max(label);
            }
        }
        index = start;
    }

    max_label + 1
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

fn file_modified_at(path: &Path) -> Option<SystemTime> {
    fs::metadata(path)
        .ok()
        .and_then(|meta| meta.modified().ok())
}

fn build_outline(document: &DocumentBuffer) -> Vec<OutlineItem> {
    document
        .blocks()
        .iter()
        .filter_map(|block| {
            let BlockKind::Heading { depth } = block.kind else {
                return None;
            };
            let title = heading_title(&document.block_text(block));
            (!title.is_empty()).then_some(OutlineItem {
                block_id: block.id,
                depth,
                title,
                source_offset: block.content_range.start,
            })
        })
        .collect()
}

fn heading_title(text: &str) -> String {
    let first_line = text.lines().next().unwrap_or(text);
    let trimmed = first_line.trim();
    let without_prefix = trimmed
        .strip_prefix("###### ")
        .or_else(|| trimmed.strip_prefix("##### "))
        .or_else(|| trimmed.strip_prefix("#### "))
        .or_else(|| trimmed.strip_prefix("### "))
        .or_else(|| trimmed.strip_prefix("## "))
        .or_else(|| trimmed.strip_prefix("# "))
        .unwrap_or(trimmed);
    without_prefix.trim_end_matches('#').trim().to_string()
}

fn remap_heading_cursor_after_markup_change(
    before: &str,
    after: &str,
    cursor_offset: usize,
) -> usize {
    let before_first_line_end = before.find('\n').unwrap_or(before.len());
    if cursor_offset > before_first_line_end {
        let after_first_line_end = after.find('\n').unwrap_or(after.len());
        let delta = after_first_line_end as isize - before_first_line_end as isize;
        return cursor_offset
            .saturating_add_signed(delta)
            .min(after.len());
    }

    let before_content_start = heading_content_start(before);
    let after_content_start = heading_content_start(after);
    let content_offset = cursor_offset.saturating_sub(before_content_start);
    (after_content_start + content_offset).min(after.len())
}

fn heading_content_start(text: &str) -> usize {
    let first_line_end = text.find('\n').unwrap_or(text.len());
    let first = &text[..first_line_end];
    let trimmed = first.trim_start();
    let indent_len = first.len().saturating_sub(trimmed.len());

    let Some(space_ix) = trimmed.find(' ') else {
        return indent_len;
    };
    let marker = &trimmed[..space_ix];
    if marker.chars().all(|ch| ch == '#') && !marker.is_empty() {
        indent_len + space_ix + 1
    } else {
        indent_len
    }
}

fn remap_list_cursor_after_markup_change(before: &str, after: &str, cursor_offset: usize) -> usize {
    let (before_line_start, before_line_end) = line_bounds_in_text(before, cursor_offset);
    let line_index = before[..before_line_start].bytes().filter(|byte| *byte == b'\n').count();
    let before_content_start = list_content_start(
        &before[before_line_start..before_line_end],
        before_line_start,
    );
    let content_offset = cursor_offset.saturating_sub(before_content_start);

    let Some((after_line_start, after_line_end)) = nth_line_bounds(after, line_index) else {
        return after.len();
    };
    let after_content_start = list_content_start(
        &after[after_line_start..after_line_end],
        after_line_start,
    );
    (after_content_start + content_offset).min(after_line_end)
}

fn list_content_start(line: &str, line_start: usize) -> usize {
    let trimmed = line.trim_start();
    let indent_len = line.len().saturating_sub(trimmed.len());
    let content_start = task_or_list_marker_len(trimmed)
        .or_else(|| ordered_marker_len(trimmed))
        .unwrap_or(0);
    line_start + indent_len + content_start
}

fn task_or_list_marker_len(trimmed: &str) -> Option<usize> {
    for marker in ["> ", ">", "- [ ] ", "- [x] ", "- [X] ", "* [ ] ", "* [x] ", "* [X] ", "+ [ ] ", "+ [x] ", "+ [X] ", "- ", "* ", "+ "] {
        if trimmed.starts_with(marker) {
            return Some(marker.len());
        }
    }
    None
}

fn ordered_marker_len(trimmed: &str) -> Option<usize> {
    let bytes = trimmed.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() && bytes[index].is_ascii_digit() {
        index += 1;
    }
    if index == 0
        || index + 1 >= bytes.len()
        || !matches!(bytes[index], b'.' | b')')
        || !bytes[index + 1].is_ascii_whitespace()
    {
        return None;
    }
    Some(index + 2)
}

fn nth_line_bounds(text: &str, target_line: usize) -> Option<(usize, usize)> {
    let mut line_start = 0usize;
    for line_index in 0..=target_line {
        if line_start > text.len() {
            return None;
        }
        let line_end = text[line_start..]
            .find('\n')
            .map(|ix| line_start + ix)
            .unwrap_or(text.len());
        if line_index == target_line {
            return Some((line_start, line_end));
        }
        line_start = line_end + 1;
    }
    None
}

fn boundary_cursor_offset(text: &str, direction: isize, preferred_column: usize) -> usize {
    let target_line = if direction >= 0 {
        0
    } else {
        text.lines().count().saturating_sub(1)
    };
    byte_offset_for_line_column(text, target_line, preferred_column)
}

fn table_cell_selection(
    block: &BlockProjection,
    table: &TableModel,
    cell_ref: TableCellRef,
) -> SelectionState {
    let source_offset = table
        .cell_source_range(cell_ref)
        .map(|range| block.content_range.start + range.start)
        .unwrap_or(block.content_range.start);
    let mut selection = SelectionState::collapsed(source_offset);
    selection.preferred_column = Some(cell_ref.column);
    selection
}

fn table_cell_selection_with_offset(
    block: &BlockProjection,
    table: &TableModel,
    cell_ref: TableCellRef,
    offset_in_cell: usize,
) -> SelectionState {
    let Some(range) = table.cell_source_range(cell_ref) else {
        return table_cell_selection(block, table, cell_ref);
    };

    let source_offset = range.start + offset_in_cell.min(range.end.saturating_sub(range.start));
    let mut selection = SelectionState::collapsed(block.content_range.start + source_offset);
    selection.preferred_column = Some(cell_ref.column);
    selection
}

fn consume_handled_action() -> EditorEffects {
    EditorEffects {
        changed: false,
        selection_changed: true,
        reload_path: None,
    }
}

#[derive(Debug, Clone)]
struct InterBlockCollapsedGap {
    replacement_range: Range<usize>,
    selection_after: usize,
}

fn inter_block_collapsed_gap(
    document: &DocumentBuffer,
    block: &BlockProjection,
) -> Option<InterBlockCollapsedGap> {
    if block.kind != BlockKind::Raw
        || !block.content_range.is_empty()
        || block.byte_range.is_empty()
        || !raw_block_is_only_whitespace(&document.block_span_text(block))
    {
        return None;
    }

    let current_index = document.block_index_by_id(block.id)?;
    let previous = current_index
        .checked_sub(1)
        .and_then(|index| document.blocks().get(index))?;
    let next = document.blocks().get(current_index + 1)?;
    if previous.kind == BlockKind::Raw || next.kind == BlockKind::Raw {
        return None;
    }

    Some(InterBlockCollapsedGap {
        replacement_range: previous.content_range.end..next.byte_range.start,
        selection_after: previous.content_range.end,
    })
}

fn supports_empty_boundary_backspace_kind(kind: &BlockKind) -> bool {
    matches!(kind, BlockKind::Raw | BlockKind::Paragraph)
}

fn supports_boundary_backspace_target_kind(kind: &BlockKind) -> bool {
    matches!(
        kind,
        BlockKind::Raw
            | BlockKind::Paragraph
            | BlockKind::Heading { .. }
            | BlockKind::List
            | BlockKind::Blockquote
            | BlockKind::Callout { .. }
    )
}

fn clamp_selection_to_text(text: &str, selection: SelectionState) -> SelectionState {
    let anchor = clamp_to_char_boundary(text, selection.anchor_byte);
    let head = clamp_to_char_boundary(text, selection.head_byte);
    SelectionState {
        anchor_byte: anchor,
        head_byte: head,
        preferred_column: selection.preferred_column,
        affinity: selection.affinity,
    }
}

fn previous_char_boundary(text: &str, offset: usize) -> usize {
    let mut cursor = clamp_to_char_boundary(text, offset);
    if cursor == 0 {
        return 0;
    }
    cursor -= 1;
    while cursor > 0 && !text.is_char_boundary(cursor) {
        cursor -= 1;
    }
    cursor
}

fn next_char_boundary(text: &str, offset: usize) -> usize {
    let mut cursor = clamp_to_char_boundary(text, offset).saturating_add(1);
    while cursor < text.len() && !text.is_char_boundary(cursor) {
        cursor += 1;
    }
    cursor.min(text.len())
}

fn auto_pair_closer_for(opener: char) -> Option<char> {
    match opener {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        '"' => Some('"'),
        '\'' => Some('\''),
        '`' => Some('`'),
        '$' => Some('$'),
        '<' => Some('>'),
        _ => None,
    }
}

fn line_bounds_in_text(text: &str, cursor_offset: usize) -> (usize, usize) {
    let cursor_offset = clamp_to_char_boundary(text, cursor_offset);
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

fn is_last_block(document: &DocumentBuffer, block: &BlockProjection) -> bool {
    document
        .block_index_by_id(block.id)
        .map(|index| index + 1 == document.blocks().len())
        .unwrap_or(false)
}

fn is_eof_empty_paragraph_block(
    document: &DocumentBuffer,
    block: &BlockProjection,
    cursor: usize,
) -> bool {
    if document.blocks().len() <= 1
        || !matches!(block.kind, BlockKind::Raw | BlockKind::Paragraph)
        || !block.content_range.is_empty()
        || !document.block_text(block).is_empty()
        || !raw_block_is_only_whitespace(&document.block_span_text(block))
        || !is_last_block(document, block)
    {
        return false;
    }

    cursor >= block.byte_range.start && cursor <= block.byte_range.end
}

fn raw_block_is_only_whitespace(text: &str) -> bool {
    text.chars()
        .all(|ch| matches!(ch, '\n' | '\r' | ' ' | '\t'))
}

fn supports_eof_empty_paragraph_predecessor_kind(kind: &BlockKind) -> bool {
    matches!(
        kind,
        BlockKind::Raw
            | BlockKind::Paragraph
            | BlockKind::Heading { .. }
            | BlockKind::List
            | BlockKind::Blockquote
    )
}

fn selection_spans_multiple_blocks(document: &DocumentBuffer, range: &Range<usize>) -> bool {
    let start_block = document.block_index_at_offset(range.start);
    let end_probe = if range.is_empty() {
        range.end
    } else {
        range.end.saturating_sub(1)
    };
    start_block != document.block_index_at_offset(end_probe)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_reflects_document_state() {
        let controller = EditorController::new(
            DocumentSource::Text {
                path: Some(PathBuf::from("note.md")),
                suggested_path: Some(PathBuf::from("note.md")),
                text: "# Title".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.path, Some(PathBuf::from("note.md")));
        assert_eq!(snapshot.display_name, "note.md");
        assert_eq!(snapshot.word_count, 1);
        assert_eq!(snapshot.document_text, "# Title");
        assert_eq!(snapshot.selection, SelectionState::collapsed(0));
        assert_eq!(snapshot.visible_selection.cursor(), 0);
        assert_eq!(snapshot.visible_selection.preferred_column, Some(0));
        assert_eq!(snapshot.display_map.visible_text, "Title");
        assert_eq!(snapshot.visible_caret_position.byte, 0);
    }

    #[test]
    fn snapshot_hides_generic_list_prefix_in_visible_text() {
        let controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "- item".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.display_map.visible_text, "item");
        assert_eq!(snapshot.visible_selection.cursor(), 0);
        assert_eq!(snapshot.visible_selection.preferred_column, Some(0));
    }

    #[test]
    fn source_mode_snapshot_uses_full_text_and_builds_outline() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "# Title\n\n## Details\nBody".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );

        controller.set_view_mode(EditorViewMode::Source);
        let snapshot = controller.snapshot();

        assert_eq!(snapshot.view_mode, EditorViewMode::Source);
        assert_eq!(
            snapshot.display_map.visible_text,
            "# Title\n\n## Details\nBody"
        );
        assert_eq!(snapshot.outline.len(), 2);
        assert_eq!(snapshot.outline[0].title, "Title");
        assert_eq!(snapshot.outline[1].title, "Details");
    }

    #[test]
    fn select_block_start_moves_selection_to_heading_offset() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "# First\n\n## Second".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );

        let second_heading = controller.snapshot().outline[1].clone();
        controller.select_block_start(second_heading.block_id);

        assert_eq!(
            controller.snapshot().selection.cursor(),
            second_heading.source_offset
        );
    }

    #[test]
    fn sync_document_state_updates_dirty_state_and_history() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Title\n".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );

        controller.dispatch(EditCommand::SyncDocumentState {
            text: "Updated title\n".to_string(),
            selection: SelectionState::collapsed(7),
        });

        let snapshot = controller.snapshot();
        assert!(snapshot.dirty);
        assert_eq!(snapshot.document_text, "Updated title\n");
        assert_eq!(snapshot.selection.cursor(), 7);

        controller.dispatch(EditCommand::Undo);
        assert_eq!(controller.snapshot().document_text, "Title\n");

        controller.dispatch(EditCommand::Redo);
        assert_eq!(controller.snapshot().document_text, "Updated title\n");
    }

    #[test]
    fn toggle_task_range_updates_checkbox_markup() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "- [ ] task".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );

        let task_range = controller
            .snapshot()
            .display_map
            .blocks
            .iter()
            .flat_map(|block| block.spans.iter())
            .find(|span| span.kind == crate::RenderSpanKind::TaskMarker)
            .expect("task marker")
            .source_range
            .clone();

        controller.toggle_task_range(task_range.clone());
        assert_eq!(controller.snapshot().document_text, "- [x] task");

        controller.toggle_task_range(task_range);
        assert_eq!(controller.snapshot().document_text, "- [ ] task");
    }

    #[test]
    fn enter_after_typed_task_marker_creates_task_list_and_next_checkbox() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "- [ ] task".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed("- [ ] task".len()),
        });

        controller.dispatch(EditCommand::InsertBreak { plain: false });

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "- [ ] task\n- [ ] ");
        assert_eq!(snapshot.selection, SelectionState::collapsed(17));
    }

    #[test]
    fn enter_after_typed_checked_task_marker_preserves_checked_state() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "* [x] done".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed("* [x] done".len()),
        });

        controller.dispatch(EditCommand::InsertBreak { plain: false });

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "* [x] done\n* [ ] ");
        assert_eq!(snapshot.selection, SelectionState::collapsed(17));
    }

    #[test]
    fn enter_after_bare_task_marker_creates_editable_task_item() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "- [ ]".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed("- [ ]".len()),
        });

        controller.dispatch(EditCommand::InsertBreak { plain: false });

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "- [ ] \n- [ ] ");
        assert_eq!(snapshot.selection, SelectionState::collapsed("- [ ] \n- [ ] ".len()));
    }

    #[test]
    fn toggle_task_list_command_converts_paragraph() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "todo".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(4),
        });

        controller.dispatch(EditCommand::ToggleTaskList);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "- [ ] todo");
        assert_eq!(snapshot.selection, SelectionState::collapsed("- [ ] todo".len()));
    }

    #[test]
    fn insert_break_splits_paragraph_and_moves_selection() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "First".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(2),
        });

        controller.dispatch(EditCommand::InsertBreak { plain: false });

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "Fi\n\nrst");
        assert_eq!(snapshot.selection, SelectionState::collapsed(4));
        assert_eq!(snapshot.blocks.len(), 2);
    }

    #[test]
    fn enter_at_eof_creates_single_trailing_empty_paragraph() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "First".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(5),
        });

        controller.dispatch(EditCommand::InsertBreak { plain: false });

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "First\n\n");
        assert_eq!(snapshot.selection, SelectionState::collapsed(7));
    }

    #[test]
    fn delete_backward_merges_trailing_empty_block() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "First\n\n".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(7),
        });

        controller.dispatch(EditCommand::DeleteBackward);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "First");
        assert_eq!(snapshot.selection, SelectionState::collapsed(5));
        assert_eq!(snapshot.blocks.len(), 1);
    }

    #[test]
    fn delete_backward_merges_trailing_empty_block_after_list() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "- item\n\n".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(8),
        });

        controller.dispatch(EditCommand::DeleteBackward);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "- item\n");
        assert_eq!(snapshot.selection, SelectionState::collapsed(7));
        assert_eq!(snapshot.blocks.len(), 1);
    }

    #[test]
    fn delete_backward_merges_trailing_empty_block_after_blockquote() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "> quote\n\n".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(9),
        });

        controller.dispatch(EditCommand::DeleteBackward);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "> quote");
        assert_eq!(snapshot.selection, SelectionState::collapsed(7));
        assert_eq!(snapshot.blocks.len(), 1);
    }

    #[test]
    fn delete_backward_between_empty_auto_pair_removes_both_sides() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Call ()".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed("Call (".len()),
        });

        controller.dispatch(EditCommand::DeleteBackward);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "Call ");
        assert_eq!(snapshot.selection, SelectionState::collapsed("Call ".len()));
    }

    #[test]
    fn delete_backward_between_empty_markdown_inline_auto_pair_removes_both_sides() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Formula $$".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed("Formula $".len()),
        });

        controller.dispatch(EditCommand::DeleteBackward);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "Formula ");
        assert_eq!(snapshot.selection, SelectionState::collapsed("Formula ".len()));
    }

    #[test]
    fn delete_backward_between_empty_angle_brackets_removes_both_sides() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Link <>".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed("Link <".len()),
        });

        controller.dispatch(EditCommand::DeleteBackward);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "Link ");
        assert_eq!(snapshot.selection, SelectionState::collapsed("Link ".len()));
    }

    #[test]
    fn delete_backward_inside_nonempty_auto_pair_removes_only_previous_char() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Call (x)".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed("Call (x".len()),
        });

        controller.dispatch(EditCommand::DeleteBackward);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "Call ()");
        assert_eq!(snapshot.selection, SelectionState::collapsed("Call (".len()));
    }

    #[test]
    fn delete_forward_before_empty_auto_pair_removes_both_sides() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Call ()".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed("Call ".len()),
        });

        controller.dispatch(EditCommand::DeleteForward);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "Call ");
        assert_eq!(snapshot.selection, SelectionState::collapsed("Call ".len()));
    }

    #[test]
    fn delete_forward_before_empty_angle_brackets_removes_both_sides() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Link <>".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed("Link ".len()),
        });

        controller.dispatch(EditCommand::DeleteForward);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "Link ");
        assert_eq!(snapshot.selection, SelectionState::collapsed("Link ".len()));
    }

    #[test]
    fn delete_forward_before_nonempty_auto_pair_removes_only_opener() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Call (x)".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed("Call ".len()),
        });

        controller.dispatch(EditCommand::DeleteForward);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "Call x)");
        assert_eq!(snapshot.selection, SelectionState::collapsed("Call ".len()));
    }

    #[test]
    fn delete_forward_merges_next_paragraph_from_heading_end() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "# Title\n\nNext".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(7),
        });

        controller.dispatch(EditCommand::DeleteForward);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "# TitleNext");
        assert_eq!(snapshot.selection, SelectionState::collapsed(7));
        assert_eq!(snapshot.blocks.len(), 1);
    }

    #[test]
    fn delete_forward_merges_next_paragraph_from_list_end() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "- item\n\nNext".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(6),
        });

        controller.dispatch(EditCommand::DeleteForward);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "- item\nNext");
        assert_eq!(snapshot.selection, SelectionState::collapsed(6));
        assert_eq!(snapshot.blocks.len(), 1);
    }

    #[test]
    fn delete_backward_demotes_heading_level_at_block_start() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "## Title".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(0),
        });

        controller.dispatch(EditCommand::DeleteBackward);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "# Title");
        assert_eq!(snapshot.selection, SelectionState::collapsed(0));
        assert_eq!(snapshot.blocks.len(), 1);
        assert!(matches!(
            snapshot.blocks[0].kind,
            crate::BlockKind::Heading { depth: 1 }
        ));
    }

    #[test]
    fn delete_backward_turns_h1_into_paragraph_at_block_start() {
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
            selection: SelectionState::collapsed(0),
        });

        controller.dispatch(EditCommand::DeleteBackward);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "Title");
        assert_eq!(snapshot.selection, SelectionState::collapsed(0));
        assert_eq!(snapshot.blocks.len(), 1);
        assert!(matches!(
            snapshot.blocks[0].kind,
            crate::BlockKind::Paragraph
        ));
    }

    #[test]
    fn toggle_heading_converts_paragraph_to_heading() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Title".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(2),
        });

        controller.dispatch(EditCommand::ToggleHeading { depth: 2 });

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "## Title");
        assert_eq!(snapshot.selection, SelectionState::collapsed("## Ti".len()));
        assert!(matches!(
            snapshot.blocks[0].kind,
            crate::BlockKind::Heading { depth: 2 }
        ));
    }

    #[test]
    fn toggle_heading_preserves_cursor_position_in_visible_title_text() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Title".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed("Ti".len()),
        });

        controller.dispatch(EditCommand::ToggleHeading { depth: 2 });

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "## Title");
        assert_eq!(snapshot.selection, SelectionState::collapsed("## Ti".len()));
    }

    #[test]
    fn toggle_heading_turns_matching_heading_back_into_paragraph() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "## Title".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(2),
        });

        controller.dispatch(EditCommand::ToggleHeading { depth: 2 });

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "Title");
        assert_eq!(snapshot.selection, SelectionState::collapsed(0));
        assert!(matches!(
            snapshot.blocks[0].kind,
            crate::BlockKind::Paragraph
        ));
    }

    #[test]
    fn toggle_heading_to_paragraph_preserves_cursor_position_in_visible_text() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "## Title".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed("## Ti".len()),
        });

        controller.dispatch(EditCommand::ToggleHeading { depth: 2 });

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "Title");
        assert_eq!(snapshot.selection, SelectionState::collapsed("Ti".len()));
    }

    #[test]
    fn toggle_heading_retargets_existing_heading_to_new_depth() {
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
            selection: SelectionState::collapsed(1),
        });

        controller.dispatch(EditCommand::ToggleHeading { depth: 3 });

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "### Title");
        assert_eq!(snapshot.selection, SelectionState::collapsed("### ".len()));
        assert!(matches!(
            snapshot.blocks[0].kind,
            crate::BlockKind::Heading { depth: 3 }
        ));
    }

    #[test]
    fn toggle_blockquote_wraps_paragraph_in_blockquote() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Hello".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(2),
        });

        controller.dispatch(EditCommand::ToggleBlockquote);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "> Hello");
        assert_eq!(snapshot.selection, SelectionState::collapsed("> He".len()));
        assert!(matches!(
            snapshot.blocks[0].kind,
            crate::BlockKind::Blockquote
        ));
    }

    #[test]
    fn toggle_blockquote_preserves_cursor_position_in_visible_text() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Hello".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed("He".len()),
        });

        controller.dispatch(EditCommand::ToggleBlockquote);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "> Hello");
        assert_eq!(snapshot.selection, SelectionState::collapsed("> He".len()));
    }

    #[test]
    fn toggle_blockquote_strips_existing_blockquote() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "> Hello".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(2),
        });

        controller.dispatch(EditCommand::ToggleBlockquote);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "Hello");
        assert_eq!(snapshot.selection, SelectionState::collapsed(0));
        assert!(matches!(
            snapshot.blocks[0].kind,
            crate::BlockKind::Paragraph
        ));
    }

    #[test]
    fn toggle_blockquote_to_paragraph_preserves_cursor_position_in_visible_text() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "> Hello".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed("> He".len()),
        });

        controller.dispatch(EditCommand::ToggleBlockquote);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "Hello");
        assert_eq!(snapshot.selection, SelectionState::collapsed("He".len()));
    }

    #[test]
    fn toggle_bullet_list_converts_paragraph_to_list() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Hello".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(2),
        });

        controller.dispatch(EditCommand::ToggleBulletList);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "- Hello");
        assert_eq!(snapshot.selection, SelectionState::collapsed("- He".len()));
        assert!(matches!(snapshot.blocks[0].kind, crate::BlockKind::List));
    }

    #[test]
    fn toggle_bullet_list_strips_existing_bullet_list() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "- Hello".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(2),
        });

        controller.dispatch(EditCommand::ToggleBulletList);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "Hello");
        assert_eq!(snapshot.selection, SelectionState::collapsed(0));
        assert!(matches!(
            snapshot.blocks[0].kind,
            crate::BlockKind::Paragraph
        ));
    }

    #[test]
    fn toggle_bullet_list_preserves_cursor_position_in_visible_item_text() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Hello".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed("He".len()),
        });

        controller.dispatch(EditCommand::ToggleBulletList);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "- Hello");
        assert_eq!(snapshot.selection, SelectionState::collapsed("- He".len()));
    }

    #[test]
    fn toggle_ordered_list_converts_paragraph_to_numbered_list() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Hello".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(2),
        });

        controller.dispatch(EditCommand::ToggleOrderedList);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "1. Hello");
        assert_eq!(snapshot.selection, SelectionState::collapsed("1. He".len()));
        assert!(matches!(snapshot.blocks[0].kind, crate::BlockKind::List));
    }

    #[test]
    fn toggle_task_list_preserves_cursor_position_in_visible_item_text() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "todo".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed("to".len()),
        });

        controller.dispatch(EditCommand::ToggleTaskList);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "- [ ] todo");
        assert_eq!(snapshot.selection, SelectionState::collapsed("- [ ] to".len()));
    }

    #[test]
    fn enter_after_typed_parenthesized_ordered_marker_continues_numbering() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "1) Hello".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed("1) Hello".len()),
        });

        controller.dispatch(EditCommand::InsertBreak { plain: false });

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "1) Hello\n2) ");
        assert_eq!(snapshot.selection, SelectionState::collapsed("1) Hello\n2) ".len()));
    }

    #[test]
    fn indent_only_adjusts_current_list_item() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "- one\n- two".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed("- one\n- two".len()),
        });

        controller.dispatch(EditCommand::Indent);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "- one\n  - two");
        assert_eq!(snapshot.selection, SelectionState::collapsed("- one\n  - two".len()));
    }

    #[test]
    fn outdent_only_adjusts_current_list_item() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "- one\n  - two".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed("- one\n  - two".len()),
        });

        controller.dispatch(EditCommand::Outdent);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "- one\n- two");
        assert_eq!(snapshot.selection, SelectionState::collapsed("- one\n- two".len()));
    }

    #[test]
    fn outdent_top_level_list_item_converts_only_current_item_to_paragraph() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "- one\n- two".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed("- one\n- two".len()),
        });

        controller.dispatch(EditCommand::Outdent);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "- one\ntwo");
        assert_eq!(snapshot.selection, SelectionState::collapsed("- one\ntwo".len()));
    }

    #[test]
    fn indent_adjusts_selected_list_items() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "- one\n- two\n- three".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState {
                anchor_byte: 0,
                head_byte: "- one\n- two".len(),
                preferred_column: None,
                affinity: SelectionAffinity::Downstream,
            },
        });

        controller.dispatch(EditCommand::Indent);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "  - one\n  - two\n- three");
        assert_eq!(snapshot.selection.range(), 0.."  - one\n  - two".len());
    }

    #[test]
    fn outdent_adjusts_selected_top_level_list_items() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "- one\n- two\n- three".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState {
                anchor_byte: 0,
                head_byte: "- one\n- two".len(),
                preferred_column: None,
                affinity: SelectionAffinity::Downstream,
            },
        });

        controller.dispatch(EditCommand::Outdent);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "one\ntwo\n- three");
        assert_eq!(snapshot.selection.range(), 0.."one\ntwo".len());
    }

    #[test]
    fn delete_backward_on_collapsed_inter_block_gap_removes_visible_empty_paragraph() {
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

        controller.dispatch(EditCommand::DeleteBackward);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "A\nB");
        assert_eq!(snapshot.selection, SelectionState::collapsed(1));
    }

    #[test]
    fn delete_forward_on_collapsed_inter_block_gap_removes_visible_empty_paragraph() {
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

        controller.dispatch(EditCommand::DeleteForward);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "A\nB");
        assert_eq!(snapshot.selection, SelectionState::collapsed(1));
    }

    #[test]
    fn enter_on_zero_length_trailing_block_upgrades_to_typora_eof_invariant() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "# A\n\n".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(5),
        });

        controller.dispatch(EditCommand::InsertBreak { plain: false });

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "# A\n\n\n\n");
        assert_eq!(snapshot.selection, SelectionState::collapsed(6));
    }

    #[test]
    fn empty_document_double_enter_inserts_typora_style_blank_paragraphs() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: String::new(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );

        controller.dispatch(EditCommand::InsertBreak { plain: false });
        controller.dispatch(EditCommand::InsertBreak { plain: false });

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "\n\n\n\n");
        assert_eq!(snapshot.selection, SelectionState::collapsed(2));
    }

    #[test]
    fn exiting_empty_list_item_at_eof_creates_single_trailing_empty_paragraph() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "- one\n- ".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(8),
        });

        controller.dispatch(EditCommand::InsertBreak { plain: false });

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "- one\n\n");
        assert_eq!(snapshot.selection, SelectionState::collapsed(7));
    }

    #[test]
    fn exiting_empty_blockquote_line_at_eof_creates_single_trailing_empty_paragraph() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "> keep\n> ".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(9),
        });

        controller.dispatch(EditCommand::InsertBreak { plain: false });

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "> keep\n\n");
        assert_eq!(snapshot.selection, SelectionState::collapsed(8));
    }

    #[test]
    fn enter_after_typed_bare_blockquote_marker_creates_empty_quote_line() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: ">".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(1),
        });

        controller.dispatch(EditCommand::InsertBreak { plain: false });

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "> \n\n");
        assert_eq!(snapshot.selection, SelectionState::collapsed(3));
    }

    #[test]
    fn enter_after_typed_blockquote_without_space_preserves_quote_text() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: ">quote".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(">quote".len()),
        });

        controller.dispatch(EditCommand::InsertBreak { plain: false });

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "> quote\n> ");
        assert_eq!(snapshot.selection, SelectionState::collapsed("> quote\n> ".len()));
    }

    #[test]
    fn enter_continues_list_inside_blockquote() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "> - one".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed("> - one".len()),
        });

        controller.dispatch(EditCommand::InsertBreak { plain: false });

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "> - one\n> - ");
        assert_eq!(snapshot.selection, SelectionState::collapsed("> - one\n> - ".len()));
    }

    #[test]
    fn enter_continues_list_inside_callout() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "> [!NOTE]\n> - one".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed("> [!NOTE]\n> - one".len()),
        });

        controller.dispatch(EditCommand::InsertBreak { plain: false });

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "> [!NOTE]\n> - one\n> - ");
        assert_eq!(
            snapshot.selection,
            SelectionState::collapsed("> [!NOTE]\n> - one\n> - ".len())
        );
    }

    #[test]
    fn indent_adjusts_list_inside_callout_body() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "> [!NOTE]\n> - one".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed("> [!NOTE]\n> - one".len()),
        });

        controller.dispatch(EditCommand::Indent);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "> [!NOTE]\n>   - one");
        assert_eq!(
            snapshot.selection,
            SelectionState::collapsed("> [!NOTE]\n>   - one".len())
        );
    }

    #[test]
    fn outdent_adjusts_nested_list_inside_callout_body() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "> [!NOTE]\n>   - one".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed("> [!NOTE]\n>   - one".len()),
        });

        controller.dispatch(EditCommand::Outdent);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "> [!NOTE]\n> - one");
        assert_eq!(
            snapshot.selection,
            SelectionState::collapsed("> [!NOTE]\n> - one".len())
        );
    }

    #[test]
    fn indent_adjusts_selected_list_items_inside_callout() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "> [!NOTE]\n> - one\n> - two\n> tail".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        let selection_start = "> [!NOTE]\n".len();
        let selection_end = "> [!NOTE]\n> - one\n> - two".len();
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState {
                anchor_byte: selection_start,
                head_byte: selection_end,
                preferred_column: None,
                affinity: SelectionAffinity::Downstream,
            },
        });

        controller.dispatch(EditCommand::Indent);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "> [!NOTE]\n>   - one\n>   - two\n> tail");
        assert_eq!(
            snapshot.selection.range(),
            selection_start.."> [!NOTE]\n>   - one\n>   - two".len()
        );
    }

    #[test]
    fn outdent_selected_top_level_list_items_inside_callout_to_quote_text() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "> [!NOTE]\n> - one\n> - two\n> tail".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        let selection_start = "> [!NOTE]\n".len();
        let selection_end = "> [!NOTE]\n> - one\n> - two".len();
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState {
                anchor_byte: selection_start,
                head_byte: selection_end,
                preferred_column: None,
                affinity: SelectionAffinity::Downstream,
            },
        });

        controller.dispatch(EditCommand::Outdent);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "> [!NOTE]\n> one\n> two\n> tail");
        assert_eq!(
            snapshot.selection.range(),
            selection_start.."> [!NOTE]\n> one\n> two".len()
        );
    }

    #[test]
    fn backspace_at_start_of_typora_eof_empty_paragraph_removes_separator_and_sentinel() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "A\n\n\n".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(3),
        });

        controller.dispatch(EditCommand::DeleteBackward);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "A");
        assert_eq!(snapshot.selection, SelectionState::collapsed(1));
    }

    #[test]
    fn backspace_on_lower_typora_eof_empty_line_removes_one_blank_line() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "A\n\n\n\n".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(4),
        });

        controller.dispatch(EditCommand::DeleteBackward);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "A\n\n\n");
        assert_eq!(snapshot.selection, SelectionState::collapsed(3));
    }

    #[test]
    fn enter_on_pipe_row_builds_table_and_places_cursor_in_first_empty_body_cell() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "| Name | Role |".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed("| Name | Role |".len()),
        });

        controller.dispatch(EditCommand::InsertBreak { plain: false });

        let snapshot = controller.snapshot();
        assert_eq!(
            snapshot.document_text,
            "| Name | Role |\n| --- | --- |\n|  |  |"
        );
        let table =
            crate::core::table::TableModel::parse("| Name | Role |\n| --- | --- |\n|  |  |");
        let first_body_cell = table
            .cell_source_range(crate::core::table::TableCellRef {
                visible_row: 1,
                column: 0,
            })
            .expect("first body cell");
        assert_eq!(snapshot.selection.cursor(), first_body_cell.start);
    }

    #[test]
    fn enter_on_loose_pipe_row_builds_table_with_outer_pipes() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Name | Role".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed("Name | Role".len()),
        });

        controller.dispatch(EditCommand::InsertBreak { plain: false });

        let snapshot = controller.snapshot();
        assert_eq!(
            snapshot.document_text,
            "| Name | Role |\n| --- | --- |\n|  |  |"
        );
        let table = crate::core::table::TableModel::parse(
            "| Name | Role |\n| --- | --- |\n|  |  |",
        );
        let first_body_cell = table
            .cell_source_range(crate::core::table::TableCellRef {
                visible_row: 1,
                column: 0,
            })
            .expect("first body cell");
        assert_eq!(snapshot.selection.cursor(), first_body_cell.start);
    }

    #[test]
    fn navigate_table_forward_appends_row_from_last_cell() {
        let source = "| Name | Role |\n| --- | --- |\n| Ada | Eng |";
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: source.to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        let table = crate::core::table::TableModel::parse(source);
        let last_cell = table
            .cell_source_range(crate::core::table::TableCellRef {
                visible_row: 1,
                column: 1,
            })
            .expect("last cell");
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(last_cell.start + "Eng".len()),
        });

        let effects = controller.navigate_table(false);
        assert!(effects.changed);

        let snapshot = controller.snapshot();
        assert_eq!(
            snapshot.document_text,
            "| Name | Role |\n| --- | --- |\n| Ada | Eng |\n|  |  |"
        );
        let rebuilt = crate::core::table::TableModel::parse(&snapshot.document_text);
        let new_row_first_cell = rebuilt
            .cell_source_range(crate::core::table::TableCellRef {
                visible_row: 2,
                column: 0,
            })
            .expect("new first cell");
        assert_eq!(snapshot.selection.cursor(), new_row_first_cell.start);
    }

    #[test]
    fn insert_table_row_rebuilds_markdown_and_places_cursor_in_new_row() {
        let source = "| Name | Role |\n| --- | --- |\n| Ada | Eng |";
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: source.to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        let body_role_cell = crate::core::table::TableModel::parse(source)
            .cell_source_range(crate::core::table::TableCellRef {
                visible_row: 1,
                column: 1,
            })
            .expect("body role cell");
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(body_role_cell.start),
        });

        let effects = controller.insert_table_row();
        assert!(effects.changed);

        let snapshot = controller.snapshot();
        let expected = "| Name | Role |\n| --- | --- |\n| Ada | Eng |\n|  |  |";
        assert_eq!(snapshot.document_text, expected);
        assert_eq!(
            snapshot.selection.cursor(),
            crate::core::table::TableModel::parse(expected)
                .cell_source_range(crate::core::table::TableCellRef {
                    visible_row: 2,
                    column: 1,
                })
                .expect("new row same column")
                .start
        );
    }

    #[test]
    fn insert_and_delete_table_column_update_markdown() {
        let source = "| Name | Role |\n| --- | --- |\n| Ada | Eng |";
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: source.to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        let body_name_cell = crate::core::table::TableModel::parse(source)
            .cell_source_range(crate::core::table::TableCellRef {
                visible_row: 1,
                column: 0,
            })
            .expect("body name cell");
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(body_name_cell.start),
        });

        let inserted = controller.insert_table_column();
        assert!(inserted.changed);
        let inserted_snapshot = controller.snapshot();
        let inserted_text = "| Name |  | Role |\n| --- | --- | --- |\n| Ada |  | Eng |";
        assert_eq!(inserted_snapshot.document_text, inserted_text);
        assert_eq!(
            inserted_snapshot.selection.cursor(),
            crate::core::table::TableModel::parse(inserted_text)
                .cell_source_range(crate::core::table::TableCellRef {
                    visible_row: 1,
                    column: 1,
                })
                .expect("inserted column cell")
                .start
        );

        let deleted = controller.delete_table_column();
        assert!(deleted.changed);
        assert_eq!(controller.snapshot().document_text, source);
    }

    #[test]
    fn insert_table_column_inherits_current_column_alignment() {
        let source = "| Name | Score |\n| --- | ---: |\n| Ada | 42 |";
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: source.to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        let body_score_cell = crate::core::table::TableModel::parse(source)
            .cell_source_range(crate::core::table::TableCellRef {
                visible_row: 1,
                column: 1,
            })
            .expect("body score cell");
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(body_score_cell.start),
        });

        let effects = controller.insert_table_column();
        assert!(effects.changed);

        let snapshot = controller.snapshot();
        let expected = "| Name | Score |  |\n| --- | ---: | ---: |\n| Ada | 42 |  |";
        assert_eq!(snapshot.document_text, expected);
        assert_eq!(
            snapshot.selection.cursor(),
            crate::core::table::TableModel::parse(expected)
                .cell_source_range(crate::core::table::TableCellRef {
                    visible_row: 1,
                    column: 2,
                })
                .expect("inserted aligned column cell")
                .start
        );
    }

    #[test]
    fn align_table_column_updates_current_column_delimiter() {
        let source = "| Name | Role |\n| --- | --- |\n| Ada | Eng |";
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: source.to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        let body_role_cell = crate::core::table::TableModel::parse(source)
            .cell_source_range(crate::core::table::TableCellRef {
                visible_row: 1,
                column: 1,
            })
            .expect("body role cell");
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(body_role_cell.start),
        });

        let effects = controller.align_table_column(TableColumnAlignment::Right);
        assert!(effects.changed);

        let snapshot = controller.snapshot();
        let expected = "| Name | Role |\n| --- | ---: |\n| Ada | Eng |";
        assert_eq!(snapshot.document_text, expected);
        assert_eq!(
            snapshot.selection.cursor(),
            crate::core::table::TableModel::parse(expected)
                .cell_source_range(crate::core::table::TableCellRef {
                    visible_row: 1,
                    column: 1,
                })
                .expect("same table cell")
                .start
        );
    }

    #[test]
    fn backspace_at_start_of_empty_table_cell_preserves_markdown() {
        let source = concat!(
            "| 1 | 2 | 3 | 4 |\n",
            "| --- | --- | --- | --- |\n",
            "|  |  |  |  |\n",
            "|  |  |  |  |"
        );
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: source.to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        let last_row_first_cell = crate::core::table::TableModel::parse(source)
            .cell_source_range(crate::core::table::TableCellRef {
                visible_row: 2,
                column: 0,
            })
            .expect("last row first cell");
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(last_row_first_cell.start),
        });

        let first = controller.dispatch(EditCommand::DeleteBackward);
        let second = controller.dispatch(EditCommand::DeleteBackward);

        assert!(!first.changed);
        assert!(first.selection_changed);
        assert!(!second.changed);
        assert!(second.selection_changed);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, source);
        assert_eq!(snapshot.selection.cursor(), last_row_first_cell.start);
    }

    #[test]
    fn delete_forward_at_end_of_empty_table_cell_preserves_markdown() {
        let source = concat!(
            "| 1 | 2 | 3 | 4 |\n",
            "| --- | --- | --- | --- |\n",
            "|  |  |  |  |\n",
            "|  |  |  |  |"
        );
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: source.to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        let last_row_first_cell = crate::core::table::TableModel::parse(source)
            .cell_source_range(crate::core::table::TableCellRef {
                visible_row: 2,
                column: 0,
            })
            .expect("last row first cell");
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(last_row_first_cell.end),
        });

        let first = controller.dispatch(EditCommand::DeleteForward);
        let second = controller.dispatch(EditCommand::DeleteForward);

        assert!(!first.changed);
        assert!(first.selection_changed);
        assert!(!second.changed);
        assert!(second.selection_changed);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, source);
        assert_eq!(snapshot.selection.cursor(), last_row_first_cell.end);
    }

    #[test]
    fn delete_table_row_removes_current_body_row_and_keeps_selection_in_table() {
        let source = concat!(
            "| Name | Role |\n",
            "| --- | --- |\n",
            "| Ada | Eng |\n",
            "| Bob | PM |\n",
            "| Cat | QA |"
        );
        let expected = concat!(
            "| Name | Role |\n",
            "| --- | --- |\n",
            "| Ada | Eng |\n",
            "| Cat | QA |"
        );
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: source.to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        let current_cell = crate::core::table::TableModel::parse(source)
            .cell_source_range(crate::core::table::TableCellRef {
                visible_row: 2,
                column: 1,
            })
            .expect("current row second cell");
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(current_cell.start),
        });

        let effects = controller.delete_table_row();
        assert!(effects.changed);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, expected);
        assert_eq!(
            snapshot.selection.cursor(),
            crate::core::table::TableModel::parse(expected)
                .cell_source_range(crate::core::table::TableCellRef {
                    visible_row: 2,
                    column: 1,
                })
                .expect("same column in following row")
                .start
        );
    }

    #[test]
    fn insert_horizontal_rule_replaces_empty_paragraph_and_creates_following_paragraph() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(0),
        });

        controller.dispatch(EditCommand::InsertHorizontalRule);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "---\n\n");
        assert_eq!(snapshot.selection, SelectionState::collapsed("---\n\n".len()));
        assert!(matches!(
            snapshot.blocks[0].kind,
            crate::BlockKind::ThematicBreak
        ));
    }

    #[test]
    fn insert_horizontal_rule_after_nonempty_paragraph_creates_following_empty_paragraph() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Hello".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(5),
        });

        controller.dispatch(EditCommand::InsertHorizontalRule);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "Hello\n\n---\n\n");
        assert!(matches!(
            snapshot.blocks[1].kind,
            crate::BlockKind::ThematicBreak
        ));
    }

    #[test]
    fn insert_code_fence_places_cursor_on_empty_body_line() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Hello".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(5),
        });

        controller.dispatch(EditCommand::InsertCodeFence);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "Hello\n\n```\n\n```");
        assert_eq!(snapshot.selection, SelectionState::collapsed(11));
        assert!(matches!(
            snapshot.blocks[1].kind,
            crate::BlockKind::CodeFence { .. }
        ));
    }

    #[test]
    fn insert_code_fence_wraps_selected_text_and_selects_body() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Before\nlet answer = 42;\nAfter".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        let start = "Before\n".len();
        let end = "Before\nlet answer = 42;".len();
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState {
                anchor_byte: start,
                head_byte: end,
                preferred_column: None,
                affinity: SelectionAffinity::Downstream,
            },
        });

        controller.dispatch(EditCommand::InsertCodeFence);

        let snapshot = controller.snapshot();
        assert_eq!(
            snapshot.document_text,
            "Before\n```\nlet answer = 42;\n```\nAfter"
        );
        assert_eq!(
            &snapshot.document_text[snapshot.selection.range()],
            "let answer = 42;"
        );
        assert!(matches!(
            snapshot.blocks[1].kind,
            crate::BlockKind::CodeFence { .. }
        ));
    }

    #[test]
    fn enter_after_typed_code_fence_preserves_language_info() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "```rust".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed("```rust".len()),
        });

        controller.dispatch(EditCommand::InsertBreak { plain: false });

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "```rust\n\n```");
        assert_eq!(snapshot.selection, SelectionState::collapsed("```rust\n".len()));
        assert!(matches!(
            snapshot.blocks[0].kind,
            crate::BlockKind::CodeFence { ref language } if language.as_deref() == Some("rust")
        ));
    }

    #[test]
    fn enter_after_typed_tilde_code_fence_uses_tilde_marker() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "~~~js".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed("~~~js".len()),
        });

        controller.dispatch(EditCommand::InsertBreak { plain: false });

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "~~~js\n\n~~~");
        assert_eq!(snapshot.selection, SelectionState::collapsed("~~~js\n".len()));
        assert!(matches!(
            snapshot.blocks[0].kind,
            crate::BlockKind::CodeFence { ref language } if language.as_deref() == Some("js")
        ));
    }

    #[test]
    fn insert_table_places_cursor_in_first_body_cell() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Hello".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(5),
        });

        controller.dispatch(EditCommand::InsertTable);

        let snapshot = controller.snapshot();
        let expected =
            "Hello\n\n| Column 1 | Column 2 | Column 3 |\n| --- | --- | --- |\n|  |  |  |";
        assert_eq!(snapshot.document_text, expected);
        let first_body_cell = crate::core::table::TableModel::parse(
            "| Column 1 | Column 2 | Column 3 |\n| --- | --- | --- |\n|  |  |  |",
        )
        .cell_source_range(crate::core::table::TableCellRef {
            visible_row: 1,
            column: 0,
        })
        .expect("first body cell");
        assert_eq!(snapshot.selection.cursor(), 7 + first_body_cell.start);
        assert!(matches!(snapshot.blocks[1].kind, crate::BlockKind::Table));
    }

    #[test]
    fn exit_table_inserts_following_empty_block_at_eof() {
        let source = concat!("| Name | Role |\n", "| --- | --- |\n", "| Ada | Eng |");
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: source.to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        let current_cell = crate::core::table::TableModel::parse(source)
            .cell_source_range(crate::core::table::TableCellRef {
                visible_row: 1,
                column: 1,
            })
            .expect("body cell");
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(current_cell.start),
        });

        let effects = controller.exit_table();
        assert!(effects.changed);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, format!("{source}\n\n"));
        assert_eq!(snapshot.selection.cursor(), source.len() + 2);
    }

    #[test]
    fn toggle_inline_markup_wraps_current_selection() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "hello".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState {
                anchor_byte: 0,
                head_byte: 5,
                preferred_column: None,
                affinity: crate::SelectionAffinity::Downstream,
            },
        });

        controller.dispatch(EditCommand::ToggleInlineMarkup {
            before: "**".to_string(),
            after: "**".to_string(),
        });

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "**hello**");
        assert_eq!(snapshot.selection.cursor(), 9);
        assert!(snapshot.selection.is_collapsed());
        assert_eq!(snapshot.visible_selection.cursor(), 5);
        assert!(snapshot.visible_selection.is_collapsed());
    }

    #[test]
    fn toggle_inline_markup_inserts_paired_markup_for_collapsed_selection() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "hello".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(5),
        });

        controller.dispatch(EditCommand::ToggleInlineMarkup {
            before: "**".to_string(),
            after: "**".to_string(),
        });

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "hello****");
        assert_eq!(snapshot.selection.cursor(), 7);
        assert!(snapshot.selection.is_collapsed());
        assert_eq!(snapshot.visible_selection.cursor(), 5);
        assert!(snapshot.visible_selection.is_collapsed());
    }

    #[test]
    fn insert_inline_math_wraps_selection() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Formula x + y".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState {
                anchor_byte: 8,
                head_byte: 13,
                preferred_column: None,
                affinity: SelectionAffinity::Downstream,
            },
        });

        controller.dispatch(EditCommand::InsertInlineMath);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "Formula $x + y$");
        assert_eq!(snapshot.selection, SelectionState::collapsed(15));
        assert!(snapshot.display_map.blocks.iter().any(|block| {
            block
                .spans
                .iter()
                .any(|span| matches!(span.meta, Some(crate::RenderSpanMeta::Math { .. })))
        }));
    }

    #[test]
    fn insert_inline_math_selects_placeholder_for_empty_selection() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Formula ".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed("Formula ".len()),
        });

        controller.dispatch(EditCommand::InsertInlineMath);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "Formula $x$");
        assert_eq!(&snapshot.document_text[snapshot.selection.range()], "x");
    }

    #[test]
    fn insert_link_wraps_selection_and_selects_url_placeholder() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Read docs".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState {
                anchor_byte: 5,
                head_byte: 9,
                preferred_column: None,
                affinity: SelectionAffinity::Downstream,
            },
        });

        controller.dispatch(EditCommand::InsertLink);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "Read [docs](https://)");
        assert_eq!(snapshot.selection.range(), 12..20);
        assert_eq!(
            &snapshot.document_text[snapshot.selection.range()],
            "https://"
        );
    }

    #[test]
    fn insert_link_escapes_selected_label() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: r"Read a\]b".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState {
                anchor_byte: 5,
                head_byte: r"Read a\]b".len(),
                preferred_column: None,
                affinity: SelectionAffinity::Downstream,
            },
        });

        controller.dispatch(EditCommand::InsertLink);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, r"Read [a\\\]b](https://)");
        assert_eq!(
            &snapshot.document_text[snapshot.selection.range()],
            "https://"
        );
    }

    #[test]
    fn insert_link_with_selected_url_uses_it_as_destination() {
        let source = "Read https://example.com/a)b";
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: source.to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState {
                anchor_byte: "Read ".len(),
                head_byte: source.len(),
                preferred_column: None,
                affinity: SelectionAffinity::Downstream,
            },
        });

        controller.dispatch(EditCommand::InsertLink);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, r"Read [text](https://example.com/a\)b)");
        assert_eq!(&snapshot.document_text[snapshot.selection.range()], "text");
    }

    #[test]
    fn insert_link_with_selected_file_url_uses_it_as_destination() {
        let source = "Open file:///tmp/report.md";
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: source.to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState {
                anchor_byte: "Open ".len(),
                head_byte: source.len(),
                preferred_column: None,
                affinity: SelectionAffinity::Downstream,
            },
        });

        controller.dispatch(EditCommand::InsertLink);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "Open [text](file:///tmp/report.md)");
        assert_eq!(&snapshot.document_text[snapshot.selection.range()], "text");
    }

    #[test]
    fn insert_link_with_collapsed_selection_creates_editable_label_and_selects_url() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Read ".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(5),
        });

        controller.dispatch(EditCommand::InsertLink);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "Read [text](https://)");
        assert_eq!(snapshot.selection.range(), 12..20);
        assert_eq!(
            &snapshot.document_text[snapshot.selection.range()],
            "https://"
        );
    }

    #[test]
    fn insert_image_placeholder_selects_path() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "See ".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(4),
        });

        controller.dispatch(EditCommand::InsertImage);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "See ![alt](image.png)");
        assert_eq!(
            &snapshot.document_text[snapshot.selection.range()],
            "image.png"
        );
    }

    #[test]
    fn insert_image_on_empty_line_creates_following_paragraph_and_selects_path() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: String::new(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );

        controller.dispatch(EditCommand::InsertImage);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "![alt](image.png)\n\n");
        assert_eq!(
            &snapshot.document_text[snapshot.selection.range()],
            "image.png"
        );
        assert!(snapshot
            .display_map
            .blocks
            .iter()
            .any(|block| block.embedded == Some(crate::EmbeddedNodeKind::Image)));
    }

    #[test]
    fn insert_image_placeholder_wraps_selection_as_escaped_alt() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: r"See a\]b".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState {
                anchor_byte: 4,
                head_byte: r"See a\]b".len(),
                preferred_column: None,
                affinity: SelectionAffinity::Downstream,
            },
        });

        controller.dispatch(EditCommand::InsertImage);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, r"See ![a\\\]b](image.png)");
        assert_eq!(
            &snapshot.document_text[snapshot.selection.range()],
            "image.png"
        );
    }

    #[test]
    fn insert_image_with_selected_url_uses_it_as_source() {
        let source = "See https://example.com/image).png";
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: source.to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState {
                anchor_byte: "See ".len(),
                head_byte: source.len(),
                preferred_column: None,
                affinity: SelectionAffinity::Downstream,
            },
        });

        controller.dispatch(EditCommand::InsertImage);

        let snapshot = controller.snapshot();
        assert_eq!(
            snapshot.document_text,
            r"See ![alt](https://example.com/image\).png)"
        );
        assert_eq!(&snapshot.document_text[snapshot.selection.range()], "alt");
    }

    #[test]
    fn insert_image_with_selected_relative_image_path_uses_it_as_source() {
        let source = "See ./assets/diagram.png";
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: source.to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState {
                anchor_byte: "See ".len(),
                head_byte: source.len(),
                preferred_column: None,
                affinity: SelectionAffinity::Downstream,
            },
        });

        controller.dispatch(EditCommand::InsertImage);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "See ![alt](./assets/diagram.png)");
        assert_eq!(&snapshot.document_text[snapshot.selection.range()], "alt");
    }

    #[test]
    fn insert_image_with_selected_spaced_image_path_escapes_destination() {
        let source = "See ./assets/my diagram(1).PNG";
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: source.to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState {
                anchor_byte: "See ".len(),
                head_byte: source.len(),
                preferred_column: None,
                affinity: SelectionAffinity::Downstream,
            },
        });

        controller.dispatch(EditCommand::InsertImage);

        let snapshot = controller.snapshot();
        assert_eq!(
            snapshot.document_text,
            r"See ![alt](./assets/my diagram(1\).PNG)"
        );
        assert_eq!(&snapshot.document_text[snapshot.selection.range()], "alt");
    }

    #[test]
    fn disk_conflict_sets_conflict_state() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: Some(PathBuf::from("note.md")),
                suggested_path: Some(PathBuf::from("note.md")),
                text: "hello\n".to_string(),
                modified_at: Some(SystemTime::UNIX_EPOCH),
            },
            SyncPolicy::default(),
        );

        controller.dispatch(EditCommand::SyncDocumentState {
            text: "draft\n".to_string(),
            selection: SelectionState::collapsed(5),
        });
        controller.apply_disk_state(
            PathBuf::from("note.md"),
            "external\n".to_string(),
            Some(SystemTime::UNIX_EPOCH + Duration::from_secs(5)),
        );

        assert!(controller.snapshot().has_conflict);
    }

    #[test]
    fn insert_math_block_on_empty_line_places_cursor_in_content_area() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: String::new(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::InsertMathBlock);

        let snapshot = controller.snapshot();
        let doc_text = &snapshot.document_text;
        assert_eq!(doc_text, "$$\n$$");
        let cursor = snapshot.selection.cursor();
        assert_eq!(cursor, 3, "source cursor should be at byte 3 (start of closing $$)");

        let visible_cursor = snapshot.visible_selection.cursor();
        let blocks = &snapshot.display_map.blocks;
        let math_block = blocks
            .iter()
            .find(|b| matches!(b.kind, BlockKind::MathBlock))
            .expect("should have a math block");
        assert!(
            visible_cursor >= math_block.visible_range.start,
            "visible cursor {} should be at or after math block start {}",
            visible_cursor,
            math_block.visible_range.start,
        );
    }

    #[test]
    fn insert_math_block_after_text_places_cursor_in_content_area() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Hello".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::InsertMathBlock);

        let snapshot = controller.snapshot();
        let doc_text = &snapshot.document_text;
        assert!(doc_text.contains("$$\n$$"), "doc should contain math block template: {doc_text}");
        let cursor = snapshot.selection.cursor();
        let math_start = doc_text.find("$$\n$$").expect("math block in doc");
        assert_eq!(cursor, math_start + 3, "source cursor should be inside math block");
    }

    #[test]
    fn insert_math_block_wraps_selected_text_and_selects_body() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Before\nx + y\nAfter".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        let start = "Before\n".len();
        let end = "Before\nx + y".len();
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState {
                anchor_byte: start,
                head_byte: end,
                preferred_column: None,
                affinity: SelectionAffinity::Downstream,
            },
        });

        controller.dispatch(EditCommand::InsertMathBlock);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "Before\n\n$$\nx + y\n$$\n\nAfter");
        assert_eq!(&snapshot.document_text[snapshot.selection.range()], "x + y");
        assert!(matches!(snapshot.blocks[1].kind, crate::BlockKind::MathBlock));
    }

    #[test]
    fn enter_after_math_block_closing_delimiter_creates_following_paragraph() {
        let source = "$$\nx + y\n$$";
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: source.to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(source.len()),
        });

        controller.dispatch(EditCommand::InsertBreak { plain: false });

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "$$\nx + y\n$$\n\n");
        assert_eq!(snapshot.selection.cursor(), snapshot.document_text.len());
    }

    #[test]
    fn enter_inside_math_block_preserves_multiline_math_editing() {
        let source = "$$\nx + y\n$$";
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: source.to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed("$$\nx".len()),
        });

        controller.dispatch(EditCommand::InsertBreak { plain: false });

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "$$\nx\n + y\n$$");
        assert_eq!(snapshot.selection.cursor(), "$$\nx\n".len());
    }

    #[test]
    fn insert_mermaid_diagram_creates_following_paragraph_and_selects_body() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: String::new(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );

        controller.dispatch(EditCommand::InsertMermaidDiagram);

        let snapshot = controller.snapshot();
        assert_eq!(
            snapshot.document_text,
            "```mermaid\ngraph TD\n  A[Start] --> B[End]\n```\n\n"
        );
        assert_eq!(
            &snapshot.document_text[snapshot.selection.range()],
            "graph TD\n  A[Start] --> B[End]"
        );
        assert!(snapshot.display_map.blocks.iter().any(|block| matches!(
            block.embedded,
            Some(crate::EmbeddedNodeKind::Diagram { ref language }) if language == "mermaid"
        )));
    }

    #[test]
    fn insert_mermaid_diagram_wraps_selected_text_and_selects_body() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Before\ngraph TD\n  A --> B\nAfter".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        let start = "Before\n".len();
        let end = "Before\ngraph TD\n  A --> B".len();
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState {
                anchor_byte: start,
                head_byte: end,
                preferred_column: None,
                affinity: SelectionAffinity::Downstream,
            },
        });

        controller.dispatch(EditCommand::InsertMermaidDiagram);

        let snapshot = controller.snapshot();
        assert_eq!(
            snapshot.document_text,
            "Before\n\n```mermaid\ngraph TD\n  A --> B\n```\n\nAfter"
        );
        assert_eq!(
            &snapshot.document_text[snapshot.selection.range()],
            "graph TD\n  A --> B"
        );
        assert!(snapshot.display_map.blocks.iter().any(|block| matches!(
            block.embedded,
            Some(crate::EmbeddedNodeKind::Diagram { ref language }) if language == "mermaid"
        )));
    }

    #[test]
    fn insert_html_block_on_empty_line_creates_following_paragraph_and_selects_content() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: String::new(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );

        controller.dispatch(EditCommand::InsertHtmlBlock);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "<div>\n  Content\n</div>\n\n");
        assert_eq!(&snapshot.document_text[snapshot.selection.range()], "Content");
        assert!(snapshot.display_map.blocks.iter().any(|block| matches!(
            block.embedded,
            Some(crate::EmbeddedNodeKind::HtmlBlock)
        )));
    }

    #[test]
    fn insert_html_block_after_text_creates_following_paragraph_and_selects_content() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Hello".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );

        controller.dispatch(EditCommand::InsertHtmlBlock);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "Hello\n\n<div>\n  Content\n</div>\n\n");
        assert_eq!(&snapshot.document_text[snapshot.selection.range()], "Content");
    }

    #[test]
    fn insert_html_block_wraps_selected_text_and_selects_body() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Before\n<p>Hi</p>\nAfter".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        let start = "Before\n".len();
        let end = "Before\n<p>Hi</p>".len();
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState {
                anchor_byte: start,
                head_byte: end,
                preferred_column: None,
                affinity: SelectionAffinity::Downstream,
            },
        });

        controller.dispatch(EditCommand::InsertHtmlBlock);

        let snapshot = controller.snapshot();
        assert_eq!(
            snapshot.document_text,
            "Before\n\n<div>\n  <p>Hi</p>\n</div>\n\nAfter"
        );
        assert_eq!(&snapshot.document_text[snapshot.selection.range()], "<p>Hi</p>");
        assert!(snapshot.display_map.blocks.iter().any(|block| matches!(
            block.embedded,
            Some(crate::EmbeddedNodeKind::HtmlBlock)
        )));
    }

    #[test]
    fn insert_callout_on_empty_line_places_cursor_in_body() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: String::new(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );

        controller.dispatch(EditCommand::InsertCallout);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "> [!NOTE] Title\n> ");
        assert_eq!(snapshot.selection.cursor(), snapshot.document_text.len());
        assert!(snapshot
            .display_map
            .blocks
            .iter()
            .any(|block| matches!(block.kind, BlockKind::Callout { .. })));
    }

    #[test]
    fn enter_after_inserted_callout_exits_to_following_paragraph() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: String::new(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );

        controller.dispatch(EditCommand::InsertCallout);
        controller.dispatch(EditCommand::InsertBreak { plain: false });

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "> [!NOTE] Title\n\n");
        assert_eq!(snapshot.selection.cursor(), snapshot.document_text.len());
    }

    #[test]
    fn insert_callout_after_text_places_cursor_in_body() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Hello".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );

        controller.dispatch(EditCommand::InsertCallout);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "Hello\n\n> [!NOTE] Title\n> ");
        assert_eq!(snapshot.selection.cursor(), snapshot.document_text.len());
    }

    #[test]
    fn insert_callout_wraps_selected_text_and_selects_body() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Before\nRemember this\nAfter".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        let start = "Before\n".len();
        let end = "Before\nRemember this".len();
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState {
                anchor_byte: start,
                head_byte: end,
                preferred_column: None,
                affinity: SelectionAffinity::Downstream,
            },
        });

        controller.dispatch(EditCommand::InsertCallout);

        let snapshot = controller.snapshot();
        assert_eq!(
            snapshot.document_text,
            "Before\n\n> [!NOTE] Title\n> Remember this\n\nAfter"
        );
        assert_eq!(
            &snapshot.document_text[snapshot.selection.range()],
            "Remember this"
        );
        assert!(snapshot
            .display_map
            .blocks
            .iter()
            .any(|block| matches!(block.kind, BlockKind::Callout { .. })));
    }

    #[test]
    fn insert_toc_on_empty_line_creates_following_paragraph() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: String::new(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );

        controller.dispatch(EditCommand::InsertToc);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "[toc]\n\n");
        assert_eq!(snapshot.selection.cursor(), "[toc]\n\n".len());
        assert!(snapshot
            .display_map
            .blocks
            .iter()
            .any(|block| block.kind == BlockKind::Toc));
        assert!(snapshot.display_map.visible_text.contains("Table of Contents"));
    }

    #[test]
    fn insert_toc_after_text_appends_toc_block_and_following_paragraph() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Hello".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );

        controller.dispatch(EditCommand::InsertToc);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "Hello\n\n[toc]\n\n");
        assert_eq!(snapshot.selection.cursor(), snapshot.document_text.len());
    }

    #[test]
    fn enter_after_typed_toc_marker_creates_following_paragraph() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "[toc]".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed("[toc]".len()),
        });

        controller.dispatch(EditCommand::InsertBreak { plain: false });

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "[toc]\n\n");
        assert_eq!(snapshot.selection.cursor(), "[toc]\n\n".len());
    }

    #[test]
    fn insert_footnote_adds_reference_and_following_paragraph_and_selects_definition_text() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Hello world".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed("Hello".len()),
        });

        controller.dispatch(EditCommand::InsertFootnote);

        let snapshot = controller.snapshot();
        assert_eq!(
            snapshot.document_text,
            "Hello[^1] world\n\n[^1]: Footnote text\n\n"
        );
        assert_eq!(
            &snapshot.document_text[snapshot.selection.range()],
            "Footnote text"
        );
        assert!(snapshot
            .display_map
            .blocks
            .iter()
            .any(|block| block.kind == BlockKind::FootnoteDefinition));
    }

    #[test]
    fn insert_footnote_moves_selected_text_into_definition() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Hello cited claim world".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState {
                anchor_byte: "Hello ".len(),
                head_byte: "Hello cited claim".len(),
                preferred_column: None,
                affinity: SelectionAffinity::Downstream,
            },
        });

        controller.dispatch(EditCommand::InsertFootnote);

        let snapshot = controller.snapshot();
        assert_eq!(
            snapshot.document_text,
            "Hello [^1] world\n\n[^1]: cited claim\n\n"
        );
        assert_eq!(
            &snapshot.document_text[snapshot.selection.range()],
            "cited claim"
        );
    }

    #[test]
    fn enter_after_typed_footnote_definition_creates_following_paragraph() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "[^1]: Note".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed("[^1]: Note".len()),
        });

        controller.dispatch(EditCommand::InsertBreak { plain: false });

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "[^1]: Note\n\n");
        assert_eq!(snapshot.selection.cursor(), "[^1]: Note\n\n".len());
    }

    #[test]
    fn enter_after_typed_link_reference_definition_creates_following_paragraph() {
        let source = "[docs]: https://example.com";
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: source.to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(source.len()),
        });

        controller.dispatch(EditCommand::InsertBreak { plain: false });

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "[docs]: https://example.com\n\n");
        assert_eq!(snapshot.selection.cursor(), snapshot.document_text.len());
        assert!(snapshot
            .display_map
            .blocks
            .iter()
            .any(|block| block.kind == BlockKind::LinkReferenceDefinition));
    }

    #[test]
    fn insert_footnote_uses_next_numeric_label() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Note[^2]\n\n[^2]: Existing".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed("Note".len()),
        });

        controller.dispatch(EditCommand::InsertFootnote);

        let snapshot = controller.snapshot();
        assert!(snapshot.document_text.contains("Note[^3][^2]"));
        assert!(snapshot.document_text.ends_with("[^3]: Footnote text\n\n"));
    }

    #[test]
    fn insert_front_matter_prepends_metadata_and_selects_title() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "# Draft".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );

        controller.dispatch(EditCommand::InsertFrontMatter);

        let snapshot = controller.snapshot();
        assert!(snapshot.document_text.starts_with(
            "---\ntitle: Untitled\ndate: \ntags: []\n---\n\n# Draft"
        ));
        assert_eq!(&snapshot.document_text[snapshot.selection.range()], "Untitled");
        assert!(snapshot
            .display_map
            .blocks
            .iter()
            .any(|block| block.kind == BlockKind::YamlFrontMatter));
    }

    #[test]
    fn insert_front_matter_uses_selected_text_as_title() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "# Draft Title\n\nBody".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState {
                anchor_byte: "# ".len(),
                head_byte: "# Draft Title".len(),
                preferred_column: None,
                affinity: SelectionAffinity::Downstream,
            },
        });

        controller.dispatch(EditCommand::InsertFrontMatter);

        let snapshot = controller.snapshot();
        assert!(snapshot.document_text.starts_with(
            "---\ntitle: Draft Title\ndate: \ntags: []\n---\n\n# Draft Title"
        ));
        assert_eq!(
            &snapshot.document_text[snapshot.selection.range()],
            "Draft Title"
        );
        assert!(snapshot
            .display_map
            .blocks
            .iter()
            .any(|block| block.kind == BlockKind::YamlFrontMatter));
    }

    #[test]
    fn insert_front_matter_does_not_duplicate_existing_metadata() {
        let source = "---\ntitle: Existing\n---\n\n# Draft";
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: source.to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );

        controller.dispatch(EditCommand::InsertFrontMatter);

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, source);
        assert_eq!(snapshot.selection, SelectionState::collapsed(0));
    }

    #[test]
    fn enter_after_typed_front_matter_creates_following_paragraph() {
        let source = "---\ntitle: Draft\n---";
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: source.to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(source.len()),
        });

        controller.dispatch(EditCommand::InsertBreak { plain: false });

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "---\ntitle: Draft\n---\n\n");
        assert_eq!(snapshot.selection.cursor(), snapshot.document_text.len());
    }

    #[test]
    fn enter_after_typed_html_block_creates_following_paragraph() {
        let source = "<div>Note</div>";
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: source.to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(source.len()),
        });

        controller.dispatch(EditCommand::InsertBreak { plain: false });

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.document_text, "<div>Note</div>\n\n");
        assert_eq!(snapshot.selection.cursor(), snapshot.document_text.len());
    }
}
