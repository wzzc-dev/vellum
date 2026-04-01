use std::{
    cmp, fs,
    ops::Range,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use anyhow::{Context as _, Result};

use super::{
    document::{BlockKind, BlockProjection, DocumentBuffer, SelectionState, Transaction},
    text_ops::{adjust_block_markup, count_document_words},
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
    pub blocks: Vec<BlockSnapshot>,
    pub active_block_id: Option<u64>,
    pub active_cursor_offset: Option<usize>,
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
    pub active_block_changed: bool,
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
    ActivateBlock(usize),
    ReplaceActiveBlock {
        text: String,
        cursor_offset: usize,
    },
    WrapActiveSelection {
        selection: Option<Range<usize>>,
        cursor_offset: usize,
        before: String,
        after: String,
        placeholder: String,
    },
    AdjustActiveBlock {
        deepen: bool,
    },
    FocusAdjacentBlock {
        direction: isize,
    },
    ExitEditMode,
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
    active_block_id: Option<u64>,
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
            active_block_id: None,
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

    pub fn snapshot(&self) -> EditorSnapshot {
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
        let active_cursor_offset = self.active_block().map(|block| {
            self.selection
                .cursor()
                .saturating_sub(block.byte_range.start)
                .min(block.byte_range.end.saturating_sub(block.byte_range.start))
        });

        EditorSnapshot {
            path: self.sync.path.clone(),
            suggested_path: self.sync.suggested_path.clone(),
            display_name: self.sync.display_name(),
            sync_state: self.sync.sync_state(),
            dirty: self.sync.dirty,
            saving: self.sync.saving,
            has_conflict: matches!(self.sync.conflict, ConflictState::Conflict { .. }),
            is_missing: self.sync.missing_on_disk,
            word_count: count_document_words(&self.document.text()),
            status_message: self.status_message.clone(),
            blocks,
            active_block_id: self.active_block_id,
            active_cursor_offset,
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
            active_block_changed: true,
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
            active_block_changed: true,
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
            active_block_changed: false,
            reload_path: None,
        })
    }

    pub fn save_as(&mut self, path: PathBuf) -> Result<EditorEffects> {
        self.sync.set_path(path.clone());
        self.save()
    }

    pub fn dispatch(&mut self, command: EditCommand) -> EditorEffects {
        match command {
            EditCommand::ActivateBlock(index) => self.activate_block(index),
            EditCommand::ReplaceActiveBlock {
                text,
                cursor_offset,
            } => self.replace_active_block(text, cursor_offset),
            EditCommand::WrapActiveSelection {
                selection,
                cursor_offset,
                before,
                after,
                placeholder,
            } => self.wrap_active_selection(selection, cursor_offset, before, after, placeholder),
            EditCommand::AdjustActiveBlock { deepen } => self.adjust_active_block(deepen),
            EditCommand::FocusAdjacentBlock { direction } => self.focus_adjacent_block(direction),
            EditCommand::ExitEditMode => self.exit_edit_mode(),
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
                        active_block_changed: false,
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
                        active_block_changed: false,
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
                        active_block_changed: false,
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
                active_block_changed: false,
                reload_path: None,
            };
        }

        if current_text == disk_text {
            self.sync.modified_at = modified_at;
            self.sync.missing_on_disk = false;
            return EditorEffects::default();
        }

        self.document = DocumentBuffer::from_text(disk_text.clone());
        self.sync
            .mark_loaded_from_disk(path.clone(), disk_text, modified_at);
        self.selection = SelectionState::collapsed(0);
        self.active_block_id = None;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.status_message = format!("Reloaded {}", path.display());

        EditorEffects {
            changed: true,
            active_block_changed: true,
            reload_path: None,
        }
    }

    fn replace_source(&mut self, source: DocumentSource) {
        *self = Self::new(source, self.sync_policy);
    }

    fn activate_block(&mut self, index: usize) -> EditorEffects {
        let Some(block) = self.document.blocks().get(index).cloned() else {
            return EditorEffects::default();
        };
        let text = self.document.block_text(&block);
        let cursor_offset = activation_cursor_offset(&text);
        self.active_block_id = Some(block.id);
        self.selection = SelectionState::collapsed(block.byte_range.start + cursor_offset);
        self.status_message = format!("Editing block {}", index + 1);
        EditorEffects {
            changed: true,
            active_block_changed: true,
            reload_path: None,
        }
    }

    fn replace_active_block(&mut self, text: String, cursor_offset: usize) -> EditorEffects {
        let Some(block) = self.active_block().cloned() else {
            return EditorEffects::default();
        };
        let cursor_offset = cmp::min(cursor_offset, text.len());
        let selection_after = SelectionState::collapsed(block.byte_range.start + cursor_offset);
        self.apply_edit(block.byte_range, text, selection_after, "Edited block")
    }

    fn wrap_active_selection(
        &mut self,
        selection: Option<Range<usize>>,
        cursor_offset: usize,
        before: String,
        after: String,
        placeholder: String,
    ) -> EditorEffects {
        let Some(block) = self.active_block().cloned() else {
            return EditorEffects::default();
        };
        let block_text = self.document.block_text(&block);
        let local_range = selection
            .filter(|range| !range.is_empty())
            .unwrap_or_else(|| {
                let clipped = cmp::min(cursor_offset, block_text.len());
                clipped..clipped
            });
        let selected_text = block_text
            .get(local_range.clone())
            .unwrap_or_default()
            .to_string();
        let insertion = if local_range.is_empty() {
            format!("{before}{placeholder}{after}")
        } else {
            format!("{before}{selected_text}{after}")
        };
        let global_range =
            block.byte_range.start + local_range.start..block.byte_range.start + local_range.end;
        let new_cursor = global_range.start + insertion.len();
        self.apply_edit(
            global_range,
            insertion,
            SelectionState::collapsed(new_cursor),
            "Updated formatting",
        )
    }

    fn adjust_active_block(&mut self, deepen: bool) -> EditorEffects {
        let Some(block) = self.active_block().cloned() else {
            return EditorEffects::default();
        };
        let current = self.document.block_text(&block);
        let Some(updated) = adjust_block_markup(&current, deepen) else {
            return EditorEffects::default();
        };
        let relative_cursor = self
            .selection
            .cursor()
            .saturating_sub(block.byte_range.start);
        let new_cursor = block.byte_range.start + cmp::min(relative_cursor, updated.len());
        self.apply_edit(
            block.byte_range,
            updated,
            SelectionState::collapsed(new_cursor),
            "Adjusted block structure",
        )
    }

    fn focus_adjacent_block(&mut self, direction: isize) -> EditorEffects {
        if self.document.blocks().is_empty() {
            return EditorEffects::default();
        }

        let current = self
            .active_block_id
            .and_then(|block_id| self.document.block_index_by_id(block_id))
            .unwrap_or(if direction >= 0 {
                0
            } else {
                self.document.blocks().len().saturating_sub(1)
            });
        let next = if direction >= 0 {
            cmp::min(current + 1, self.document.blocks().len().saturating_sub(1))
        } else {
            current.saturating_sub(1)
        };
        self.activate_block(next)
    }

    fn exit_edit_mode(&mut self) -> EditorEffects {
        if self.active_block_id.is_none() {
            return EditorEffects::default();
        }
        self.active_block_id = None;
        self.status_message = "Selection cleared".to_string();
        EditorEffects {
            changed: true,
            active_block_changed: true,
            reload_path: None,
        }
    }

    fn undo(&mut self) -> EditorEffects {
        let Some(entry) = self.undo_stack.pop() else {
            return EditorEffects::default();
        };

        self.document.apply_transaction(Transaction::Replace {
            range: entry.after_range.clone(),
            replacement: entry.before_text.clone(),
        });
        self.selection = entry.selection_before.clone();
        self.active_block_id = self
            .document
            .blocks()
            .get(self.document.block_index_at_offset(self.selection.cursor()))
            .map(|block| block.id);
        self.sync.mark_document_changed(&self.document.text());
        self.redo_stack.push(entry);
        self.status_message = "Undo".to_string();
        EditorEffects {
            changed: true,
            active_block_changed: true,
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
        self.selection = entry.selection_after.clone();
        self.active_block_id = self
            .document
            .blocks()
            .get(self.document.block_index_at_offset(self.selection.cursor()))
            .map(|block| block.id);
        self.sync.mark_document_changed(&self.document.text());
        self.undo_stack.push(entry);
        self.status_message = "Redo".to_string();
        EditorEffects {
            changed: true,
            active_block_changed: true,
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

        self.document = DocumentBuffer::from_text(disk_text.clone());
        self.sync
            .mark_loaded_from_disk(path, disk_text, modified_at);
        self.selection = SelectionState::collapsed(0);
        self.active_block_id = None;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.status_message = "Reloaded disk version".to_string();

        EditorEffects {
            changed: true,
            active_block_changed: true,
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
            active_block_changed: false,
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
        self.active_block_id = self
            .document
            .blocks()
            .get(self.document.block_index_at_offset(self.selection.cursor()))
            .map(|block| block.id);
        self.sync.mark_document_changed(&self.document.text());
        self.status_message = status_message.to_string();

        EditorEffects {
            changed: true,
            active_block_changed: true,
            reload_path: None,
        }
    }

    fn active_block(&self) -> Option<&BlockProjection> {
        self.active_block_id
            .and_then(|block_id| self.document.block_by_id(block_id))
            .or_else(|| {
                self.document
                    .blocks()
                    .get(self.document.block_index_at_offset(self.selection.cursor()))
            })
    }
}

fn file_modified_at(path: &Path) -> Option<SystemTime> {
    fs::metadata(path)
        .ok()
        .and_then(|meta| meta.modified().ok())
}

fn activation_cursor_offset(text: &str) -> usize {
    text.trim_end_matches(['\r', '\n']).len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_reflects_document_state() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: Some(PathBuf::from("note.md")),
                suggested_path: Some(PathBuf::from("note.md")),
                text: "hello world".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::ActivateBlock(0));
        controller.status_message = "Testing".to_string();

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.path, Some(PathBuf::from("note.md")));
        assert_eq!(snapshot.display_name, "note.md");
        assert_eq!(snapshot.word_count, 2);
        assert_eq!(snapshot.status_message, "Testing");
        assert!(snapshot.active_block_id.is_some());
    }

    #[test]
    fn replace_command_updates_dirty_state_and_history() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: "Title\n".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );

        controller.dispatch(EditCommand::ActivateBlock(0));
        controller.dispatch(EditCommand::ReplaceActiveBlock {
            text: "Updated title\n".to_string(),
            cursor_offset: 7,
        });

        let snapshot = controller.snapshot();
        assert!(snapshot.dirty);
        assert_eq!(snapshot.blocks[0].text, "Updated title");

        controller.dispatch(EditCommand::Undo);
        assert_eq!(controller.snapshot().blocks[0].text, "Title");

        controller.dispatch(EditCommand::Redo);
        assert_eq!(controller.snapshot().blocks[0].text, "Updated title");
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

        controller.dispatch(EditCommand::ActivateBlock(0));
        controller.dispatch(EditCommand::ReplaceActiveBlock {
            text: "draft\n".to_string(),
            cursor_offset: 5,
        });
        controller.apply_disk_state(
            PathBuf::from("note.md"),
            "external\n".to_string(),
            Some(SystemTime::UNIX_EPOCH + Duration::from_secs(5)),
        );

        assert!(controller.snapshot().has_conflict);
    }

    #[test]
    fn workspace_relocation_requests_reload_for_current_document() {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: Some(PathBuf::from("old.md")),
                suggested_path: Some(PathBuf::from("old.md")),
                text: "hello\n".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );

        let effects = controller.apply_file_event(FileSyncEvent::Relocated {
            from: PathBuf::from("old.md"),
            to: PathBuf::from("new.md"),
        });

        assert_eq!(effects.reload_path, Some(PathBuf::from("new.md")));
        assert_eq!(controller.snapshot().path, Some(PathBuf::from("new.md")));
    }
}
