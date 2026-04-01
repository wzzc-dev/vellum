use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
};

use anyhow::{Context as _, Result};
use gpui_component::tree::TreeItem;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher, event::ModifyKind};

#[derive(Debug, Clone)]
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
}

fn map_workspace_event(event: Event) -> WorkspaceEvent {
    if matches!(event.kind, EventKind::Modify(ModifyKind::Name(_))) && event.paths.len() >= 2 {
        return WorkspaceEvent::Relocated {
            from: event.paths[0].clone(),
            to: event.paths[1].clone(),
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
