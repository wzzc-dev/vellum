use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};

use crate::manifest::{ComponentKind, VellumManifest};
use crate::runtime::{
    EditorCommandRequest, LoadedComponent, PluginAction, PluginActionKind, VellumAppRuntime,
};
use crate::ui::{
    AppEvent, CommandEvent, EditorSnapshot, PluginCommand, PluginInfo, PluginPanel, PluginState,
    UiEvent, ViewTree,
};

pub struct PluginStore {
    runtime: VellumAppRuntime,
    plugins: Vec<PluginEntry>,
    dev_plugin_paths: Vec<PathBuf>,
}

struct PluginEntry {
    directory: PathBuf,
    manifest: VellumManifest,
    component: Option<LoadedComponent>,
    disabled: bool,
    error: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct DevPluginsFile {
    #[serde(default)]
    plugins: Vec<DevPluginRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DevPluginRecord {
    path: String,
}

impl PluginStore {
    pub fn new() -> Result<Self> {
        Ok(Self {
            runtime: VellumAppRuntime::new()?,
            plugins: Vec::new(),
            dev_plugin_paths: Vec::new(),
        })
    }

    pub fn load_dev_plugins_file(&mut self, path: &Path) -> Result<Vec<String>> {
        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = toml::from_str::<DevPluginsFile>(&std::fs::read_to_string(path)?)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        self.dev_plugin_paths = file
            .plugins
            .into_iter()
            .map(|plugin| PathBuf::from(plugin.path))
            .collect();

        let mut loaded = Vec::new();
        for plugin_path in self.dev_plugin_paths.clone() {
            match self.load_plugin_dir(&plugin_path) {
                Ok(id) => loaded.push(id),
                Err(err) => eprintln!("failed to load dev plugin {}: {err}", plugin_path.display()),
            }
        }
        Ok(loaded)
    }

    pub fn save_dev_plugins_file(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = DevPluginsFile {
            plugins: self
                .dev_plugin_paths
                .iter()
                .map(|path| DevPluginRecord {
                    path: path.to_string_lossy().to_string(),
                })
                .collect(),
        };
        std::fs::write(path, toml::to_string_pretty(&file)?)?;
        Ok(())
    }

    pub fn install_dev_plugin(
        &mut self,
        directory: PathBuf,
        registry_path: &Path,
    ) -> Result<String> {
        let id = self.load_plugin_dir(&directory)?;
        if !self.dev_plugin_paths.iter().any(|path| path == &directory) {
            self.dev_plugin_paths.push(directory);
            self.save_dev_plugins_file(registry_path)?;
        }
        Ok(id)
    }

    pub fn discover_in_dir(&mut self, root: &Path) -> Result<Vec<String>> {
        if !root.exists() {
            return Ok(Vec::new());
        }

        let mut loaded = Vec::new();
        if root.join("vellum.toml").exists() {
            loaded.push(self.load_plugin_dir(root)?);
            return Ok(loaded);
        }

        for entry in std::fs::read_dir(root)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() && path.join("vellum.toml").exists() {
                match self.load_plugin_dir(&path) {
                    Ok(id) => loaded.push(id),
                    Err(err) => {
                        eprintln!("failed to discover plugin {}: {err}", path.display());
                    }
                }
            }
        }
        Ok(loaded)
    }

    pub fn plugin_infos(&self) -> Vec<PluginInfo> {
        self.plugins.iter().map(PluginEntry::plugin_info).collect()
    }

    pub fn panel_ids(&self) -> Vec<(String, String)> {
        self.plugins
            .iter()
            .filter(|entry| !entry.disabled && entry.component.is_some())
            .flat_map(|entry| {
                entry.manifest.contributes.panels.iter().map(|panel| {
                    (
                        qualified_id(&entry.manifest.id, &panel.id),
                        panel.title.clone(),
                    )
                })
            })
            .collect()
    }

    pub fn set_host_context(&mut self, snapshot: EditorSnapshot, plugins: Vec<PluginInfo>) {
        for entry in &mut self.plugins {
            if let Some(component) = entry.component.as_mut() {
                component.set_editor_snapshot(snapshot.clone());
                component.set_plugin_infos(plugins.clone());
            }
        }
    }

    pub fn enable(&mut self, id: &str) -> Result<()> {
        let index = self
            .plugins
            .iter()
            .position(|entry| entry.manifest.id == id)
            .with_context(|| format!("plugin not found: {id}"))?;
        self.plugins[index].disabled = false;
        self.reload_at(index)
    }

    pub fn disable(&mut self, id: &str) -> Result<()> {
        let entry = self
            .plugins
            .iter_mut()
            .find(|entry| entry.manifest.id == id)
            .with_context(|| format!("plugin not found: {id}"))?;
        if let Some(component) = entry.component.as_mut() {
            let _ = component.shutdown();
        }
        entry.component = None;
        entry.disabled = true;
        entry.error = None;
        Ok(())
    }

    pub fn reload(&mut self, id: &str) -> Result<()> {
        let index = self
            .plugins
            .iter()
            .position(|entry| entry.manifest.id == id)
            .with_context(|| format!("plugin not found: {id}"))?;
        self.reload_at(index)
    }

    pub fn execute_command(&mut self, command_id: &str) -> Result<()> {
        let Some((plugin_id, local_id)) = split_qualified_id(command_id) else {
            anyhow::bail!("plugin command id must be qualified: {command_id}");
        };
        let entry = self
            .plugins
            .iter_mut()
            .find(|entry| entry.manifest.id == plugin_id)
            .with_context(|| format!("plugin not found: {plugin_id}"))?;
        if entry.disabled {
            anyhow::bail!("plugin is disabled: {plugin_id}");
        }
        let Some(component) = entry.component.as_mut() else {
            anyhow::bail!("plugin is not loaded: {plugin_id}");
        };
        component.update(AppEvent::Command(CommandEvent {
            command_id: local_id.to_string(),
            payload: Vec::new(),
        }))?;
        Ok(())
    }

    pub fn panel_tree(&mut self, panel_id: &str) -> Option<ViewTree> {
        let (plugin_id, _local_id) = split_qualified_id(panel_id)?;
        self.plugins
            .iter_mut()
            .find(|entry| {
                entry.manifest.id == plugin_id && !entry.disabled && entry.component.is_some()
            })
            .and_then(|entry| {
                entry
                    .component
                    .as_mut()
                    .and_then(|component| component.view_tree().cloned())
            })
    }

    pub fn dispatch_ui_event_to_panel(&mut self, panel_id: &str, event: UiEvent) -> Result<()> {
        let Some((plugin_id, _local_id)) = split_qualified_id(panel_id) else {
            anyhow::bail!("plugin panel id must be qualified: {panel_id}");
        };
        let entry = self
            .plugins
            .iter_mut()
            .find(|entry| entry.manifest.id == plugin_id)
            .with_context(|| format!("plugin not found: {plugin_id}"))?;
        if entry.disabled {
            return Ok(());
        }
        if let Some(component) = entry.component.as_mut() {
            component.update(AppEvent::Ui(event))?;
        }
        Ok(())
    }

    pub fn dispatch_tick(&mut self, tick: u64) {
        for entry in &mut self.plugins {
            if entry.disabled {
                continue;
            }
            if let Some(component) = entry.component.as_mut() {
                if let Err(err) = component.update(AppEvent::Tick(tick)) {
                    entry.error = Some(err.to_string());
                }
            }
        }
    }

    pub fn drain_editor_commands(&mut self) -> Vec<EditorCommandRequest> {
        let mut commands = Vec::new();
        for entry in &mut self.plugins {
            if let Some(component) = entry.component.as_mut() {
                commands.extend(component.take_editor_commands());
            }
        }
        commands
    }

    pub fn drain_plugin_actions(&mut self) -> Vec<PluginAction> {
        let mut actions = Vec::new();
        for entry in &mut self.plugins {
            if let Some(component) = entry.component.as_mut() {
                actions.extend(component.take_plugin_actions());
            }
        }
        actions
    }

    pub fn apply_plugin_action(&mut self, action: PluginAction) -> Result<()> {
        match action.kind {
            PluginActionKind::Enable => self.enable(&action.id),
            PluginActionKind::Disable => self.disable(&action.id),
            PluginActionKind::Reload => self.reload(&action.id),
        }
    }

    fn load_plugin_dir(&mut self, directory: &Path) -> Result<String> {
        let manifest_path = directory.join("vellum.toml");
        let manifest = VellumManifest::from_toml_bytes(&std::fs::read(&manifest_path)?)
            .with_context(|| format!("failed to parse {}", manifest_path.display()))?;
        if manifest.kind != ComponentKind::Plugin {
            anyhow::bail!("manifest is not a plugin: {}", manifest.id);
        }

        if let Some(index) = self
            .plugins
            .iter()
            .position(|entry| entry.manifest.id == manifest.id)
        {
            if let Some(component) = self.plugins[index].component.as_mut() {
                let _ = component.shutdown();
            }
            self.plugins.remove(index);
        }

        let mut entry = PluginEntry {
            directory: directory.to_path_buf(),
            manifest,
            component: None,
            disabled: false,
            error: None,
        };
        let id = entry.manifest.id.clone();
        if let Err(err) = entry.load(&self.runtime) {
            entry.error = Some(err.to_string());
        }
        self.plugins.push(entry);
        Ok(id)
    }

    fn reload_at(&mut self, index: usize) -> Result<()> {
        let entry = &mut self.plugins[index];
        if let Some(component) = entry.component.as_mut() {
            let _ = component.shutdown();
        }
        entry.component = None;
        entry.error = None;
        if entry.disabled {
            return Ok(());
        }
        if let Err(err) = entry.load(&self.runtime) {
            entry.error = Some(err.to_string());
        }
        Ok(())
    }
}

impl PluginEntry {
    fn load(&mut self, runtime: &VellumAppRuntime) -> Result<()> {
        let mut component =
            runtime.load_component(self.directory.clone(), self.manifest.clone())?;
        component.init()?;
        self.component = Some(component);
        Ok(())
    }

    fn plugin_info(&self) -> PluginInfo {
        let state = if self.disabled {
            PluginState::Disabled
        } else if self.error.is_some() {
            PluginState::Failed
        } else {
            PluginState::Enabled
        };

        PluginInfo {
            id: self.manifest.id.clone(),
            name: self.manifest.name.clone(),
            version: self.manifest.version.clone(),
            description: self.manifest.description.clone(),
            state,
            commands: self
                .manifest
                .contributes
                .commands
                .iter()
                .map(|command| PluginCommand {
                    id: qualified_id(&self.manifest.id, &command.id),
                    title: command.title.clone(),
                })
                .collect(),
            panels: self
                .manifest
                .contributes
                .panels
                .iter()
                .map(|panel| PluginPanel {
                    id: qualified_id(&self.manifest.id, &panel.id),
                    title: panel.title.clone(),
                })
                .collect(),
            error: self.error.clone(),
        }
    }
}

fn qualified_id(plugin_id: &str, local_id: &str) -> String {
    if local_id.starts_with(plugin_id) {
        local_id.to_string()
    } else {
        format!("{plugin_id}.{local_id}")
    }
}

fn split_qualified_id(id: &str) -> Option<(&str, &str)> {
    id.rsplit_once('.')
}
