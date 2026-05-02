use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use serde::{Serialize, Deserialize};

use crate::ui::UiNode;

/// 完整的扩展状态快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionFullState {
    pub extension_id: String,
    pub timestamp: u64,
    
    // UI 状态
    pub panel_uis: HashMap<String, UiNodeState>,
    
    // 计时器状态
    pub tick_enabled: bool,
    pub tick_interval_ms: Option<u32>,
    
    // 环境变量（自定义）
    pub custom_state: serde_json::Value,
}

/// UI 节点状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UiNodeState {
    Serialized(serde_json::Value),
}

impl From<&UiNode> for UiNodeState {
    fn from(_node: &UiNode) -> Self {
        UiNodeState::Serialized(serde_json::Value::Null)
    }
}

/// 状态持久化器
pub struct StatePersister {
    save_dir: PathBuf,
}

impl StatePersister {
    pub fn new(save_dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&save_dir);
        Self { save_dir }
    }

    /// 保存状态到文件
    pub fn save_to_file(&self, state: &ExtensionFullState) -> Result<PathBuf> {
        let filename = format!("{}_{}.state.json", state.extension_id, state.timestamp);
        let path = self.save_dir.join(filename);
        let json = serde_json::to_string_pretty(state)?;
        std::fs::write(&path, json)?;
        Ok(path)
    }

    /// 从文件加载状态
    pub fn load_latest(&self, extension_id: &str) -> Result<Option<ExtensionFullState>> {
        let mut latest: Option<(u64, ExtensionFullState)> = None;
        
        if let Ok(entries) = std::fs::read_dir(&self.save_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let (Some(ext), Some(stem)) = (path.extension(), path.file_stem()) {
                    if ext == "json" {
                        let stem_str = stem.to_string_lossy();
                        if stem_str.starts_with(&format!("{extension_id}_")) {
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                if let Ok(state) = serde_json::from_str::<ExtensionFullState>(&content) {
                                    if latest.map(|(t, _)| t < state.timestamp).unwrap_or(true) {
                                        latest = Some((state.timestamp, state));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(latest.map(|(_, state)| state))
    }
}
