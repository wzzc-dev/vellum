pub mod watch;
pub mod builder;
pub mod state;
pub mod settings;

use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::Result;

use crate::host::{ExtensionHost, PanelId};
use crate::registry::ExtensionEntry;

pub use watch::{ExtensionFileWatcher, FileChangeEvent};
pub use builder::{BuildController, BuildResult};
pub use settings::HotReloadSettings;

/// 热重载状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReloadState {
    Idle,
    Building,
    SavingState,
    Unloading,
    Loading,
    RestoringState,
    Complete,
    Failed,
}

/// 热重载控制器
pub struct HotReloadController {
    file_watcher: ExtensionFileWatcher,
    build_controller: BuildController,
    watch_state: Arc<Mutex<WatchState>>,
    settings: HotReloadSettings,
}

struct WatchState {
    reload_state: ReloadState,
    last_error: Option<String>,
    auto_reload: bool,
}

impl HotReloadController {
    pub fn new() -> Self {
        Self {
            file_watcher: ExtensionFileWatcher::new(),
            build_controller: BuildController::new(),
            watch_state: Arc::new(Mutex::new(WatchState {
                reload_state: ReloadState::Idle,
                last_error: None,
                auto_reload: true,
            })),
            settings: HotReloadSettings::default(),
        }
    }

    pub fn with_settings(settings: HotReloadSettings) -> Self {
        Self {
            file_watcher: ExtensionFileWatcher::new(),
            build_controller: BuildController::new(),
            watch_state: Arc::new(Mutex::new(WatchState {
                reload_state: ReloadState::Idle,
                last_error: None,
                auto_reload: settings.auto_reload,
            })),
            settings,
        }
    }

    /// 开始监听开发扩展
    pub fn start_watching(&mut self, extension_id: &str, directory: &Path) -> Result<()> {
        self.file_watcher.watch_extension(extension_id, directory)
    }

    /// 检查并处理变更（可调用的方法）
    pub async fn check_and_reload(&mut self, extension_host: &mut ExtensionHost, extension_id: &str) -> Result<ReloadResult> {
        let changes = self.file_watcher.check_changes(extension_id);
        
        if changes.is_empty() {
            return Ok(ReloadResult::NoChanges);
        }

        let entry = extension_host.registry().get(extension_id).cloned();
        let Some(entry) = entry else {
            return Ok(ReloadResult::Error("Extension not found".to_string()));
        };

        self.perform_reload(extension_host, &entry).await
    }

    /// 执行完整的热重载
    async fn perform_reload(&mut self, extension_host: &mut ExtensionHost, entry: &ExtensionEntry) -> Result<ReloadResult> {
        self.set_state(ReloadState::Building);
        
        let build_result = self.build_controller
            .build_extension(&entry.manifest.id, &entry.directory)
            .await?;
        
        match build_result {
            BuildResult::Success { wasm_path: _, duration_ms } => {
                self.set_state(ReloadState::SavingState);
                let saved_state = if self.settings.preserve_state {
                    Some(ExtensionStateSaver::save(extension_host, &entry.manifest.id)?)
                } else {
                    None
                };
                
                self.set_state(ReloadState::Unloading);
                let was_loaded = extension_host.is_extension_loaded(&entry.manifest.id);
                
                if was_loaded {
                    extension_host.unload_extension(&entry.manifest.id)?;
                }
                
                self.set_state(ReloadState::Loading);
                match extension_host.activate_extension(&entry.manifest.id) {
                    Ok(_) => {
                        self.set_state(ReloadState::RestoringState);
                        if let Some(saved_state) = saved_state {
                            ExtensionStateSaver::restore(extension_host, &entry.manifest.id, saved_state)?;
                        }
                        
                        self.set_state(ReloadState::Complete);
                        
                        Ok(ReloadResult::Success {
                            duration_ms,
                        })
                    }
                    Err(e) => {
                        self.set_state(ReloadState::Failed);
                        self.set_last_error(e.to_string());
                        Ok(ReloadResult::Error(e.to_string()))
                    }
                }
            }
            BuildResult::Failure { error_message, duration_ms } => {
                self.set_state(ReloadState::Failed);
                self.set_last_error(error_message.clone());
                Ok(ReloadResult::BuildFailed {
                    error: error_message,
                    duration_ms,
                })
            }
        }
    }

    /// 获取当前状态
    pub fn get_state(&self) -> ReloadState {
        self.watch_state.lock().unwrap().reload_state
    }

    /// 获取最后一个错误
    pub fn get_last_error(&self) -> Option<String> {
        self.watch_state.lock().unwrap().last_error.clone()
    }

    /// 设置自动重载
    pub fn set_auto_reload(&mut self, enabled: bool) {
        self.watch_state.lock().unwrap().auto_reload = enabled;
        self.settings.auto_reload = enabled;
    }

    /// 获取设置
    pub fn settings(&self) -> &HotReloadSettings {
        &self.settings
    }

    /// 获取可变设置
    pub fn settings_mut(&mut self) -> &mut HotReloadSettings {
        &mut self.settings
    }

    fn set_state(&self, state: ReloadState) {
        let mut watch_state = self.watch_state.lock().unwrap();
        watch_state.reload_state = state;
    }

    fn set_last_error(&self, error: String) {
        let mut watch_state = self.watch_state.lock().unwrap();
        watch_state.last_error = Some(error);
    }
}

impl Default for HotReloadController {
    fn default() -> Self {
        Self::new()
    }
}

/// 热重载结果
pub enum ReloadResult {
    Success {
        duration_ms: u64,
    },
    NoChanges,
    BuildFailed {
        error: String,
        duration_ms: u64,
    },
    Error(String),
}

/// 扩展状态保存/恢复
struct ExtensionStateSaver;

impl ExtensionStateSaver {
    /// 保存扩展状态
    pub fn save(extension_host: &ExtensionHost, extension_id: &str) -> Result<ExtensionSavedState> {
        let panel_uis: Vec<_> = extension_host.panel_uis()
            .iter()
            .filter(|(id, _)| id.starts_with(&format!("{extension_id}.")))
            .map(|(id, node)| (id.clone(), node.clone()))
            .collect();
        
        Ok(ExtensionSavedState {
            panel_uis,
            timestamp: std::time::SystemTime::now(),
        })
    }

    /// 恢复扩展状态
    pub fn restore(extension_host: &mut ExtensionHost, extension_id: &str, state: ExtensionSavedState) -> Result<()> {
        for (panel_id, ui_node) in state.panel_uis {
            extension_host.set_panel_view(panel_id, ui_node);
        }
        Ok(())
    }
}

/// 保存的扩展状态
struct ExtensionSavedState {
    panel_uis: Vec<(PanelId, crate::ui::UiNode)>,
    timestamp: std::time::SystemTime,
}
