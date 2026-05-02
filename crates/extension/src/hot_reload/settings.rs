use serde::{Deserialize, Serialize};

/// Hot Reload 设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotReloadSettings {
    /// 是否启用自动重载
    #[serde(default = "default_true")]
    pub auto_reload: bool,
    
    /// 防抖时间（毫秒），在最后一次变更后等待
    #[serde(default = "default_debounce_ms")]
    pub debounce_ms: u64,
    
    /// 是否显示重载通知
    #[serde(default = "default_true")]
    pub show_notifications: bool,
    
    /// 最大重载重试次数
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    
    /// 是否保留状态
    #[serde(default = "default_true")]
    pub preserve_state: bool,
    
    /// 是否在失败时回滚
    #[serde(default = "default_true")]
    pub rollback_on_failure: bool,
}

fn default_true() -> bool { true }
fn default_debounce_ms() -> u64 { 300 }
fn default_max_retries() -> u32 { 3 }

impl Default for HotReloadSettings {
    fn default() -> Self {
        Self {
            auto_reload: default_true(),
            debounce_ms: default_debounce_ms(),
            show_notifications: default_true(),
            max_retries: default_max_retries(),
            preserve_state: default_true(),
            rollback_on_failure: default_true(),
        }
    }
}
