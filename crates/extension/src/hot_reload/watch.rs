use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use notify::{EventKind, RecursiveMode, Watcher as NotifyWatcher};
use tokio::sync::mpsc;

/// 文件变更事件类型
#[derive(Debug, Clone)]
pub enum FileChangeEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Removed(PathBuf),
}

/// 文件监听器
pub struct ExtensionFileWatcher {
    watchers: HashMap<String, (notify::RecommendedWatcher, mpsc::UnboundedReceiver<FileChangeEvent>)>,
    extension_paths: HashMap<String, Vec<PathBuf>>,
}

impl ExtensionFileWatcher {
    pub fn new() -> Self {
        Self {
            watchers: HashMap::new(),
            extension_paths: HashMap::new(),
        }
    }

    /// 开始监听一个扩展的目录
    pub fn watch_extension(&mut self, extension_id: &str, directory: &Path) -> Result<()> {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut watcher = notify::recommended_watcher(move |res| {
            if let Ok(event) = res {
                if let Some(path) = event.paths.first() {
                    let change = match event.kind {
                        EventKind::Create(_) => FileChangeEvent::Created(path.clone()),
                        EventKind::Modify(_) => FileChangeEvent::Modified(path.clone()),
                        EventKind::Remove(_) => FileChangeEvent::Removed(path.clone()),
                        _ => return,
                    };
                    let _ = tx.send(change);
                }
            }
        })?;
        watcher.watch(directory, RecursiveMode::Recursive)?;
        
        self.watchers.insert(extension_id.to_string(), (watcher, rx));
        self.extension_paths.insert(extension_id.to_string(), vec![directory.to_path_buf()]);
        
        Ok(())
    }

    /// 停止监听
    pub fn unwatch_extension(&mut self, extension_id: &str) {
        self.watchers.remove(extension_id);
        self.extension_paths.remove(extension_id);
    }

    /// 检查是否有变更（非阻塞）
    pub fn check_changes(&mut self, extension_id: &str) -> Vec<FileChangeEvent> {
        let mut changes = Vec::new();
        if let Some((_, rx)) = self.watchers.get_mut(extension_id) {
            while let Ok(event) = rx.try_recv() {
                changes.push(event);
            }
        }
        changes
    }
}

impl Default for ExtensionFileWatcher {
    fn default() -> Self {
        Self::new()
    }
}
