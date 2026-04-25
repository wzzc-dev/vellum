use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use wasmi::{Instance, Store};

use crate::abi::{HostState, PendingEdit};
use crate::decoration::{Decoration, OverlayPanel, RegisteredPanel, Tooltip};
use crate::event::{EventData, EventType};
use crate::manifest::PluginManifest;
use crate::memory;
use crate::protocol;
use crate::runtime::PluginRuntime;
use crate::ui::{UiEvent, UiNode};

struct LoadedPlugin {
    manifest: PluginManifest,
    instance: Instance,
    store: Store<HostState>,
    memory: wasmi::Memory,
    subscribed_events: Vec<EventType>,
}

pub struct PluginManager {
    runtime: PluginRuntime,
    loaded_plugins: Vec<LoadedPlugin>,
    commands: Vec<crate::command::RegisteredCommand>,
    sidebar_panels: Vec<RegisteredPanel>,
    panel_uis: HashMap<u32, UiNode>,
    decorations: Vec<Decoration>,
    active_overlay: Option<OverlayPanel>,
    active_tooltip: Option<Tooltip>,
    pending_status_message: Option<String>,
    pending_edits: Vec<PendingEdit>,
    document_text: String,
    document_path: Option<String>,
}

impl PluginManager {
    pub fn new() -> Result<Self> {
        let runtime = PluginRuntime::new()?;
        Ok(Self {
            runtime,
            loaded_plugins: Vec::new(),
            commands: Vec::new(),
            sidebar_panels: Vec::new(),
            panel_uis: HashMap::new(),
            decorations: Vec::new(),
            active_overlay: None,
            active_tooltip: None,
            pending_status_message: None,
            pending_edits: Vec::new(),
            document_text: String::new(),
            document_path: None,
        })
    }

    pub fn load_plugin(&mut self, wasm_path: &Path) -> Result<PluginManifest> {
        let wasm_bytes = std::fs::read(wasm_path)
            .with_context(|| format!("failed to read WASM file: {}", wasm_path.display()))?;

        let module = self.runtime.load_module(&wasm_bytes)?;
        let plugin_id = wasm_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let state = HostState::new(plugin_id);
        let mut store = self.runtime.create_store(state);

        let instance_pre = self
            .runtime
            .linker()
            .instantiate(&mut store, &module)
            .with_context(|| {
                let mut missing = Vec::new();
                for import in module.imports() {
                    missing.push(format!("{}::{}", import.module(), import.name()));
                }
                format!("failed to instantiate WASM module (imports: {:?})", missing)
            })?;

        let instance = instance_pre
            .start(&mut store)
            .context("failed to start WASM module")?;

        let mem = memory::get_memory(&instance, &store)?;

        let manifest = Self::call_manifest(&instance, &mut store, &mem)?;

        if self.loaded_plugins.iter().any(|p| p.manifest.id == manifest.id) {
            anyhow::bail!("plugin {} is already loaded", manifest.id);
        }

        let plugin_id_for_state = manifest.id.clone();

        store.data_mut().plugin_id = plugin_id_for_state;

        Self::call_init(&instance, &mut store)?;

        let new_commands = store.data_mut().take_commands();
        let new_panels = store.data_mut().take_panels();
        let new_subscriptions = store.data_mut().take_subscriptions();

        let subscribed_events: Vec<EventType> = new_subscriptions
            .iter()
            .filter_map(|(event_type, _)| EventType::try_from(*event_type).ok())
            .collect();

        self.commands.extend(new_commands);
        self.sidebar_panels.extend(new_panels);

        let loaded = LoadedPlugin {
            manifest: manifest.clone(),
            instance,
            store,
            memory: mem,
            subscribed_events,
        };
        self.loaded_plugins.push(loaded);

        Ok(manifest)
    }

    pub fn load_plugins_from_dir(&mut self, dir: &Path) -> Result<Vec<PluginManifest>> {
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut manifests = Vec::new();
        let entries = std::fs::read_dir(dir)
            .with_context(|| format!("failed to read plugin directory: {}", dir.display()))?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("wasm") {
                match self.load_plugin(&path) {
                    Ok(manifest) => {
                        manifests.push(manifest);
                    }
                    Err(e) => {
                        eprintln!("failed to load plugin {:?}: {}", path, e);
                    }
                }
            }
        }
        Ok(manifests)
    }

    pub fn unload_plugin(&mut self, plugin_id: &str) -> Result<()> {
        if let Some(pos) = self.loaded_plugins.iter().position(|p| p.manifest.id == plugin_id) {
            let mut plugin = self.loaded_plugins.remove(pos);
            Self::call_shutdown_internal(&mut plugin);
            self.commands.retain(|c| c.plugin_id != plugin_id);
            self.sidebar_panels.retain(|p| p.plugin_id != plugin_id);
        }
        Ok(())
    }

    pub fn dispatch_event(&mut self, event: EventData) {
        let event_type = event.event_type();
        let encoded = protocol::encode_event_data(&event);

        for i in 0..self.loaded_plugins.len() {
            let is_subscribed = self.loaded_plugins[i].subscribed_events.contains(&event_type);
            if !is_subscribed {
                continue;
            }

            let plugin = &mut self.loaded_plugins[i];
            if let Ok(func) = plugin.instance.get_typed_func::<(u32, u32, u32), ()>(
                &plugin.store,
                "plugin_handle_event",
            ) {
                let data_ptr = Self::write_to_plugin_memory_internal(plugin, &encoded);
                let _ = func.call(&mut plugin.store, (event_type as u32, data_ptr, encoded.len() as u32));
            }

            let plugin = &mut self.loaded_plugins[i];
            Self::collect_plugin_state_internal(plugin, &mut self.panel_uis, &mut self.decorations, &mut self.active_overlay, &mut self.active_tooltip, &mut self.pending_status_message, &mut self.pending_edits);
        }
    }

    pub fn execute_command(&mut self, command_id: u32) -> bool {
        for i in 0..self.loaded_plugins.len() {
            let plugin = &mut self.loaded_plugins[i];
            if let Ok(func) = plugin.instance.get_typed_func::<u32, ()>(
                &plugin.store,
                "plugin_execute_command",
            ) {
                let _ = func.call(&mut plugin.store, command_id);
                Self::collect_plugin_state_internal(plugin, &mut self.panel_uis, &mut self.decorations, &mut self.active_overlay, &mut self.active_tooltip, &mut self.pending_status_message, &mut self.pending_edits);
                return true;
            }
        }
        false
    }

    pub fn handle_ui_event(&mut self, event: UiEvent) {
        let encoded = protocol::encode_ui_event(&event);

        for i in 0..self.loaded_plugins.len() {
            let plugin = &mut self.loaded_plugins[i];
            if let Ok(func) = plugin.instance.get_typed_func::<(u32, u32), ()>(
                &plugin.store,
                "plugin_handle_ui_event",
            ) {
                let data_ptr = Self::write_to_plugin_memory_internal(plugin, &encoded);
                let _ = func.call(&mut plugin.store, (data_ptr, encoded.len() as u32));
            }

            let plugin = &mut self.loaded_plugins[i];
            Self::collect_plugin_state_internal(plugin, &mut self.panel_uis, &mut self.decorations, &mut self.active_overlay, &mut self.active_tooltip, &mut self.pending_status_message, &mut self.pending_edits);
        }
    }

    pub fn handle_hover(&mut self, hover_data: &str) -> Option<Tooltip> {
        let hover_bytes = hover_data.as_bytes();

        for i in 0..self.loaded_plugins.len() {
            let plugin = &mut self.loaded_plugins[i];
            if let Ok(func) = plugin.instance.get_typed_func::<(u32, u32), u64>(
                &plugin.store,
                "plugin_handle_hover",
            ) {
                let data_ptr = Self::write_to_plugin_memory_internal(plugin, hover_bytes);
                let result = func.call(&mut plugin.store, (data_ptr, hover_bytes.len() as u32));
                if let Ok(packed) = result {
                    if packed != 0 {
                        let ptr = (packed >> 32) as u32;
                        let len = (packed & 0xFFFFFFFF) as u32;
                        let bytes = memory::read_memory(&plugin.memory, &plugin.store, ptr, len);
                        if let Ok(tooltip) = protocol::decode_tooltip(&bytes) {
                            return tooltip;
                        }
                    }
                }
            }
        }
        None
    }

    pub fn update_document(&mut self, text: String, path: Option<String>) {
        self.document_text = text.clone();
        self.document_path = path.clone();
        for plugin in &mut self.loaded_plugins {
            plugin.store.data_mut().update_document(text.clone(), path.clone());
        }
    }

    pub fn commands(&self) -> &[crate::command::RegisteredCommand] {
        &self.commands
    }

    pub fn sidebar_panels(&self) -> &[RegisteredPanel] {
        &self.sidebar_panels
    }

    pub fn panel_ui(&self, panel_id: u32) -> Option<&UiNode> {
        self.panel_uis.get(&panel_id)
    }

    pub fn decorations(&self) -> &[Decoration] {
        &self.decorations
    }

    pub fn active_overlay(&self) -> Option<&OverlayPanel> {
        self.active_overlay.as_ref()
    }

    pub fn take_status_message(&mut self) -> Option<String> {
        std::mem::take(&mut self.pending_status_message)
    }

    pub fn take_edits(&mut self) -> Vec<PendingEdit> {
        std::mem::take(&mut self.pending_edits)
    }

    pub fn shutdown_all(&mut self) {
        for plugin in &mut self.loaded_plugins {
            Self::call_shutdown_internal(plugin);
        }
        self.loaded_plugins.clear();
    }

    pub fn loaded_manifests(&self) -> Vec<PluginManifest> {
        self.loaded_plugins.iter().map(|p| p.manifest.clone()).collect()
    }

    fn call_manifest(
        instance: &Instance,
        store: &mut Store<HostState>,
        mem: &wasmi::Memory,
    ) -> Result<PluginManifest> {
        let func = instance
            .get_typed_func::<(), u64>(&*store, "plugin_manifest")
            .context("WASM module does not export 'plugin_manifest'")?;

        let result = func.call(&mut *store, ()).context("failed to call plugin_manifest")?;
        let ptr = (result >> 32) as u32;
        let len = (result & 0xFFFFFFFF) as u32;

        let bytes = memory::read_memory(mem, &*store, ptr, len);
        protocol::decode_manifest(&bytes)
    }

    fn call_init(instance: &Instance, store: &mut Store<HostState>) -> Result<()> {
        let func = instance
            .get_typed_func::<(), ()>(&*store, "plugin_init")
            .context("WASM module does not export 'plugin_init'")?;

        func.call(&mut *store, ()).context("failed to call plugin_init")
    }

    fn call_shutdown_internal(plugin: &mut LoadedPlugin) {
        if let Ok(func) = plugin.instance.get_typed_func::<(), ()>(&plugin.store, "plugin_shutdown")
        {
            let _ = func.call(&mut plugin.store, ());
        }
    }

    fn write_to_plugin_memory_internal(plugin: &mut LoadedPlugin, data: &[u8]) -> u32 {
        let ptr = plugin.store.data_mut().alloc_offset;
        plugin.store.data_mut().alloc_offset += data.len() as u32;

        if memory::write_memory(&plugin.memory, &mut plugin.store, ptr, data).is_ok() {
            ptr
        } else {
            0
        }
    }

    fn collect_plugin_state_internal(
        plugin: &mut LoadedPlugin,
        panel_uis: &mut HashMap<u32, UiNode>,
        decorations: &mut Vec<Decoration>,
        active_overlay: &mut Option<OverlayPanel>,
        active_tooltip: &mut Option<Tooltip>,
        status_message: &mut Option<String>,
        pending_edits: &mut Vec<PendingEdit>,
    ) {
        let state = plugin.store.data_mut();

        if let Some(msg) = state.take_status_message() {
            *status_message = Some(msg);
        }

        for (panel_id, ui) in state.panel_uis.drain() {
            panel_uis.insert(panel_id, ui);
        }

        if !state.decorations.is_empty() {
            *decorations = std::mem::take(&mut state.decorations);
        }

        if state.active_overlay.is_some() {
            *active_overlay = state.active_overlay.take();
        }

        if state.active_tooltip.is_some() {
            *active_tooltip = state.active_tooltip.take();
        }

        let edits = state.take_edits();
        if !edits.is_empty() {
            pending_edits.extend(edits);
        }
    }
}
