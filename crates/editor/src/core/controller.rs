use std::{
    cmp, fs,
    ops::Range,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use anyhow::{Context as _, Result};

use super::{
    display_map::DisplayMap,
    document::{BlockKind, BlockProjection, DocumentBuffer, SelectionState, Transaction},
    text_ops::{
        adjust_block_markup, byte_offset_for_line_column, clamp_to_char_boundary,
        compute_document_diff, count_document_words, line_column_for_byte_offset,
        semantic_enter_transform,
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
    Indent,
    Outdent,
    MoveCaret {
        direction: isize,
        preferred_column: Option<usize>,
    },
    DeleteBackward,
    DeleteForward,
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
        }
    }

    pub fn from_disk(path: PathBuf, sync_policy: SyncPolicy) -> Result<Self> {
        Ok(Self::new(DocumentSource::from_disk(path)?, sync_policy))
    }

    pub fn autosave_delay(&self) -> Duration {
        self.sync_policy.autosave_delay
    }
}

impl EditorController {
    pub fn snapshot(&self) -> EditorSnapshot {
        let document_text = self.document.text();
        let selection = clamp_selection_to_text(&document_text, self.selection.clone());
        let caret_byte = selection.cursor().min(document_text.len());
        let (line, column) = line_column_for_byte_offset(&document_text, caret_byte);
        let display_map = self.document.display_map(Some(&selection));
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
        }
    }

    pub fn document_path(&self) -> Option<&PathBuf> {
        self.sync.path.as_ref()
    }

    pub fn current_document_dir(&self) -> Option<PathBuf> {
        self.sync.current_dir()
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
            EditCommand::Indent => self.adjust_current_block(true),
            EditCommand::Outdent => self.adjust_current_block(false),
            EditCommand::MoveCaret {
                direction,
                preferred_column,
            } => self.move_caret_to_adjacent_block(direction, preferred_column),
            EditCommand::DeleteBackward => self.delete_backward(),
            EditCommand::DeleteForward => self.delete_forward(),
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
        *self = Self::new(source, self.sync_policy);
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
        if plain {
            let start = self.selection.range().start;
            let selection_after = SelectionState::collapsed(start + 1);
            return self.replace_selection_with_text(
                "\n".to_string(),
                Some(selection_after),
                "Inserted line break",
            );
        }

        let range = self.selection.range();
        if selection_spans_multiple_blocks(&self.document, &range) {
            let selection_after = SelectionState::collapsed(range.start + 1);
            return self.replace_selection_with_text(
                "\n".to_string(),
                Some(selection_after),
                "Inserted line break",
            );
        }

        let Some(block) = self.current_block().cloned() else {
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

    fn adjust_current_block(&mut self, deepen: bool) -> EditorEffects {
        let Some(block) = self.current_block().cloned() else {
            return EditorEffects::default();
        };
        let current = self.document.block_text(&block);
        let Some(updated) = adjust_block_markup(&current, deepen) else {
            return EditorEffects::default();
        };
        let relative_cursor = self
            .selection
            .cursor()
            .saturating_sub(block.content_range.start);
        let new_cursor = block.content_range.start + cmp::min(relative_cursor, updated.len());
        self.apply_edit(
            block.content_range,
            updated,
            SelectionState::collapsed(new_cursor),
            "Adjusted block structure",
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

    fn delete_backward(&mut self) -> EditorEffects {
        if !self.selection.is_collapsed() {
            let selection_after = SelectionState::collapsed(self.selection.range().start);
            return self.replace_selection_with_text(
                String::new(),
                Some(selection_after),
                "Deleted selection",
            );
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

        if let Some(effect) = self.delete_forward_eof_empty_paragraph() {
            return effect;
        }
        if let Some(effect) = self.delete_forward_collapsed_inter_block_gap() {
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

fn file_modified_at(path: &Path) -> Option<SystemTime> {
    fs::metadata(path)
        .ok()
        .and_then(|meta| meta.modified().ok())
}

fn boundary_cursor_offset(text: &str, direction: isize, preferred_column: usize) -> usize {
    let target_line = if direction >= 0 {
        0
    } else {
        text.lines().count().saturating_sub(1)
    };
    byte_offset_for_line_column(text, target_line, preferred_column)
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
        BlockKind::Raw | BlockKind::Paragraph | BlockKind::Heading { .. }
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
}
