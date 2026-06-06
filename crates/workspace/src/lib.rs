use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
};

use anyhow::{Context as _, Result};
use gpui_component::tree::TreeItem;
use notify::{
    Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
    event::{ModifyKind, RenameMode},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceEvent {
    Changed(PathBuf),
    Removed(PathBuf),
    Relocated { from: PathBuf, to: PathBuf },
    Unknown,
}

pub struct WorkspaceState {
    pub root: Option<PathBuf>,
    pub expanded_dirs: BTreeSet<PathBuf>,
    pub selected_file: Option<PathBuf>,
    watcher: Option<RecommendedWatcher>,
    rx: Option<Receiver<WorkspaceEvent>>,
}

impl WorkspaceState {
    pub fn new() -> Self {
        Self {
            root: None,
            expanded_dirs: BTreeSet::new(),
            selected_file: None,
            watcher: None,
            rx: None,
        }
    }

    pub fn set_root(&mut self, root: Option<PathBuf>) -> Result<()> {
        self.root = root.clone();
        self.expanded_dirs.clear();
        self.selected_file = None;
        self.watcher = None;
        self.rx = None;

        if let Some(root) = root {
            self.expanded_dirs.insert(root.clone());

            let (tx, rx) = mpsc::channel();
            let mut watcher = notify::recommended_watcher(move |result: notify::Result<Event>| {
                let event = match result {
                    Ok(event) => map_workspace_event(event),
                    Err(_) => WorkspaceEvent::Unknown,
                };
                let _ = tx.send(event);
            })
            .context("failed to create file watcher")?;
            watcher
                .watch(&root, RecursiveMode::Recursive)
                .with_context(|| format!("failed to watch {}", root.display()))?;
            self.watcher = Some(watcher);
            self.rx = Some(rx);
        }

        Ok(())
    }

    pub fn poll_events(&mut self) -> Vec<WorkspaceEvent> {
        let mut events = Vec::new();
        let Some(rx) = &self.rx else {
            return events;
        };

        while let Ok(event) = rx.try_recv() {
            events.push(event);
        }

        events
    }

    pub fn tree_items(&self) -> Result<Vec<TreeItem>> {
        let Some(root) = &self.root else {
            return Ok(Vec::new());
        };
        Ok(vec![build_tree_item(root, &self.expanded_dirs)?])
    }

    pub fn tree_items_matching(&self, query: &str) -> Result<Vec<TreeItem>> {
        let Some(root) = &self.root else {
            return Ok(Vec::new());
        };
        let query = normalize_query(query);
        if query.is_empty() {
            return self.tree_items();
        }
        Ok(build_filtered_tree_item(root, root, &query)?.into_iter().collect())
    }
}

fn map_workspace_event(event: Event) -> WorkspaceEvent {
    if let EventKind::Modify(ModifyKind::Name(mode)) = event.kind {
        return match (mode, event.paths.as_slice()) {
            (RenameMode::Both, [from, to, ..]) => WorkspaceEvent::Relocated {
                from: from.clone(),
                to: to.clone(),
            },
            (RenameMode::From, [path, ..]) => WorkspaceEvent::Removed(path.clone()),
            (RenameMode::To, [path, ..]) => WorkspaceEvent::Changed(path.clone()),
            (_, [path, ..]) => WorkspaceEvent::Changed(path.clone()),
            _ => WorkspaceEvent::Unknown,
        };
    }

    let path = event.paths.first().cloned();
    match (event.kind, path) {
        (EventKind::Modify(_), Some(path))
        | (EventKind::Create(_), Some(path))
        | (EventKind::Any, Some(path)) => WorkspaceEvent::Changed(path),
        (EventKind::Remove(_), Some(path)) => WorkspaceEvent::Removed(path),
        _ => WorkspaceEvent::Unknown,
    }
}

fn build_tree_item(path: &Path, expanded_dirs: &BTreeSet<PathBuf>) -> Result<TreeItem> {
    let label = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.to_string_lossy().to_string());
    let id = path.to_string_lossy().to_string();

    if path.is_dir() {
        let mut entries = fs::read_dir(path)
            .with_context(|| format!("failed to read {}", path.display()))?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                let entry_path = entry.path();
                entry_path.is_dir() || is_markdown_path(&entry_path)
            })
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| compare_paths(&left.path(), &right.path()));

        let children = entries
            .iter()
            .map(|entry| build_tree_item(&entry.path(), expanded_dirs))
            .collect::<Result<Vec<_>>>()?;

        Ok(TreeItem::new(id, label)
            .expanded(expanded_dirs.contains(path))
            .children(children))
    } else {
        Ok(TreeItem::new(id, label))
    }
}

fn build_filtered_tree_item(path: &Path, root: &Path, query: &str) -> Result<Option<TreeItem>> {
    let label = path_label(path);
    let id = path.to_string_lossy().to_string();

    if path.is_dir() {
        let mut entries = markdown_tree_entries(path)?;
        entries.sort_by(|left, right| compare_paths(&left.path(), &right.path()));

        let direct_match = path_matches_query(path, root, &label, query);
        let children = if direct_match {
            entries
                .iter()
                .map(|entry| build_expanded_tree_item(&entry.path()))
                .collect::<Result<Vec<_>>>()?
        } else {
            entries
                .iter()
                .filter_map(|entry| match build_filtered_tree_item(&entry.path(), root, query) {
                    Ok(Some(item)) => Some(Ok(item)),
                    Ok(None) => None,
                    Err(err) => Some(Err(err)),
                })
                .collect::<Result<Vec<_>>>()?
        };

        if direct_match || !children.is_empty() {
            Ok(Some(TreeItem::new(id, label).expanded(true).children(children)))
        } else {
            Ok(None)
        }
    } else if path_matches_query(path, root, &label, query) {
        Ok(Some(TreeItem::new(id, label)))
    } else {
        Ok(None)
    }
}

fn build_expanded_tree_item(path: &Path) -> Result<TreeItem> {
    let label = path_label(path);
    let id = path.to_string_lossy().to_string();

    if path.is_dir() {
        let mut entries = markdown_tree_entries(path)?;
        entries.sort_by(|left, right| compare_paths(&left.path(), &right.path()));
        let children = entries
            .iter()
            .map(|entry| build_expanded_tree_item(&entry.path()))
            .collect::<Result<Vec<_>>>()?;
        Ok(TreeItem::new(id, label).expanded(true).children(children))
    } else {
        Ok(TreeItem::new(id, label))
    }
}

fn markdown_tree_entries(path: &Path) -> Result<Vec<fs::DirEntry>> {
    Ok(fs::read_dir(path)
        .with_context(|| format!("failed to read {}", path.display()))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            let entry_path = entry.path();
            entry_path.is_dir() || is_markdown_path(&entry_path)
        })
        .collect::<Vec<_>>())
}

fn path_label(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

fn path_matches_query(path: &Path, root: &Path, label: &str, query: &str) -> bool {
    label.to_ascii_lowercase().contains(query)
        || path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/")
            .to_ascii_lowercase()
            .contains(query)
}

fn normalize_query(query: &str) -> String {
    query.trim().replace('\\', "/").to_ascii_lowercase()
}

fn compare_paths(left: &Path, right: &Path) -> std::cmp::Ordering {
    match (left.is_dir(), right.is_dir()) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => left
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .cmp(
                &right
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or_default(),
            ),
    }
}

pub fn is_markdown_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            ext.eq_ignore_ascii_case("md")
                || ext.eq_ignore_ascii_case("markdown")
                || ext.eq_ignore_ascii_case("mdown")
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempWorkspace {
        root: PathBuf,
    }

    impl TempWorkspace {
        fn new() -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after epoch")
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "vellum-workspace-filter-{}-{unique}",
                std::process::id()
            ));
            fs::create_dir_all(&root).expect("temp workspace should be created");
            Self { root }
        }

        fn write(&self, path: &str, contents: &str) {
            let path = self.root.join(path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("temp workspace parent should be created");
            }
            fs::write(path, contents).expect("temp workspace file should be written");
        }

        fn state(&self) -> WorkspaceState {
            WorkspaceState {
                root: Some(self.root.clone()),
                expanded_dirs: BTreeSet::new(),
                selected_file: None,
                watcher: None,
                rx: None,
            }
        }
    }

    impl Drop for TempWorkspace {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn child_labels(item: &TreeItem) -> Vec<String> {
        item.children
            .iter()
            .map(|child| child.label.to_string())
            .collect()
    }

    fn name_event(mode: RenameMode, paths: &[&str]) -> Event {
        paths.iter().fold(
            Event::new(EventKind::Modify(ModifyKind::Name(mode))),
            |event, path| event.add_path(PathBuf::from(path)),
        )
    }

    #[test]
    fn maps_paired_rename_to_relocation() {
        assert_eq!(
            map_workspace_event(name_event(RenameMode::Both, &["old.md", "new.md"])),
            WorkspaceEvent::Relocated {
                from: PathBuf::from("old.md"),
                to: PathBuf::from("new.md"),
            }
        );
    }

    #[test]
    fn maps_single_rename_from_to_removed() {
        assert_eq!(
            map_workspace_event(name_event(RenameMode::From, &["old.md"])),
            WorkspaceEvent::Removed(PathBuf::from("old.md"))
        );
    }

    #[test]
    fn maps_single_rename_to_to_changed() {
        assert_eq!(
            map_workspace_event(name_event(RenameMode::To, &["new.md"])),
            WorkspaceEvent::Changed(PathBuf::from("new.md"))
        );
    }

    #[test]
    fn maps_unclassified_rename_with_path_to_changed() {
        assert_eq!(
            map_workspace_event(name_event(RenameMode::Any, &["note.md"])),
            WorkspaceEvent::Changed(PathBuf::from("note.md"))
        );
    }

    #[test]
    fn filters_markdown_files_across_collapsed_directories() {
        let workspace = TempWorkspace::new();
        workspace.write("Draft.md", "# Draft");
        workspace.write("notes/Meeting Notes.markdown", "# Meeting");
        workspace.write("notes/ignore.txt", "not shown");

        let items = workspace
            .state()
            .tree_items_matching("meeting")
            .expect("filtered tree should build");

        assert_eq!(items.len(), 1);
        assert!(items[0].is_expanded());
        assert_eq!(child_labels(&items[0]), vec!["notes"]);
        let notes = &items[0].children[0];
        assert!(notes.is_expanded());
        assert_eq!(child_labels(notes), vec!["Meeting Notes.markdown"]);
    }

    #[test]
    fn file_filter_matches_case_insensitive_relative_paths() {
        let workspace = TempWorkspace::new();
        workspace.write("deep/sub/plan.mdown", "# Plan");
        workspace.write("deep/sub/other.md", "# Other");
        workspace.write("root.md", "# Root");

        let items = workspace
            .state()
            .tree_items_matching("SUB/PLAN")
            .expect("filtered tree should build");

        let root = &items[0];
        assert_eq!(child_labels(root), vec!["deep"]);
        let deep = &root.children[0];
        assert_eq!(child_labels(deep), vec!["sub"]);
        let sub = &deep.children[0];
        assert_eq!(child_labels(sub), vec!["plan.mdown"]);
    }

    #[test]
    fn matching_folder_expands_markdown_subtree() {
        let workspace = TempWorkspace::new();
        workspace.write("chapters/one.md", "# One");
        workspace.write("chapters/nested/two.md", "# Two");
        workspace.write("outside.md", "# Outside");

        let items = workspace
            .state()
            .tree_items_matching("chapters")
            .expect("filtered tree should build");

        let root = &items[0];
        assert_eq!(child_labels(root), vec!["chapters"]);
        let chapters = &root.children[0];
        assert!(chapters.is_expanded());
        assert_eq!(child_labels(chapters), vec!["nested", "one.md"]);
        assert_eq!(child_labels(&chapters.children[0]), vec!["two.md"]);
    }
}
