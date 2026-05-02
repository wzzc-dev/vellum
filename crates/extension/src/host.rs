use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::ResourceTable;
use wasmtime_wasi::p2::{IoView, WasiCtx, WasiCtxBuilder, WasiView};

use crate::contributions::{
    Decoration, PendingEdit, RegisteredCommand, RegisteredPanel, Tooltip, VersionedPayload,
};
use crate::event::ExtensionEvent;
use crate::manifest::ExtensionManifest;
use crate::permissions::{Capability, check_capability};
use crate::registry::{ExtensionRegistry, ExtensionState};
use crate::ui::{UiEvent, UiNode};

#[allow(dead_code)]
mod bindings {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "extension-world",
    });
}

use bindings::ExtensionWorld;
use bindings::vellum::extension::types::{
    ActivationContext, ExtensionError, ExtensionEvent as WitExtensionEvent, HostError, LogLevel,
    UiEvent as WitUiEvent,
};

pub type PanelId = String;

#[derive(Debug, Clone, Default)]
pub struct ExtensionOutputs {
    pub status_message: Option<String>,
    pub pending_edits: Vec<PendingEdit>,
    pub decorations: Option<Vec<Decoration>>,
    pub panel_uis: HashMap<PanelId, UiNode>,
}

impl ExtensionOutputs {
    fn merge(&mut self, other: ExtensionOutputs) {
        if other.status_message.is_some() {
            self.status_message = other.status_message;
        }
        self.pending_edits.extend(other.pending_edits);
        if other.decorations.is_some() {
            self.decorations = other.decorations;
        }
        self.panel_uis.extend(other.panel_uis);
    }
}

pub struct ExtensionRuntimeState {
    extension_id: String,
    manifest: ExtensionManifest,
    wasi_ctx: WasiCtx,
    resource_table: ResourceTable,
    document_text: String,
    document_path: Option<String>,
    outputs: ExtensionOutputs,
    tick_interval_ms: Option<u32>,
    tick_next_due_ms: Option<u64>,
}

impl ExtensionRuntimeState {
    fn new(extension_id: String, manifest: ExtensionManifest) -> Self {
        Self {
            extension_id,
            manifest,
            wasi_ctx: WasiCtxBuilder::new().build(),
            resource_table: ResourceTable::new(),
            document_text: String::new(),
            document_path: None,
            outputs: ExtensionOutputs::default(),
            tick_interval_ms: None,
            tick_next_due_ms: None,
        }
    }

    fn set_document(&mut self, text: String, path: Option<String>) {
        self.document_text = text;
        self.document_path = path;
    }

    fn take_outputs(&mut self) -> ExtensionOutputs {
        std::mem::take(&mut self.outputs)
    }

    fn permission_error(&self, capability: Capability) -> HostError {
        HostError {
            message: format!(
                "extension '{}' does not have '{}' capability",
                self.extension_id,
                capability.name()
            ),
        }
    }

    fn require(&self, capability: Capability) -> std::result::Result<(), HostError> {
        check_capability(&self.manifest, capability).map_err(|_| self.permission_error(capability))
    }
}

impl IoView for ExtensionRuntimeState {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.resource_table
    }
}

impl WasiView for ExtensionRuntimeState {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi_ctx
    }
}

impl bindings::vellum::extension::types::Host for ExtensionRuntimeState {}

impl bindings::vellum::extension::host::Host for ExtensionRuntimeState {
    fn log(&mut self, level: LogLevel, message: String) {
        eprintln!("[extension:{}:{level:?}] {message}", self.extension_id);
    }

    fn show_status_message(&mut self, message: String) -> std::result::Result<(), HostError> {
        self.outputs.status_message = Some(message);
        Ok(())
    }
}

impl bindings::vellum::extension::editor::Host for ExtensionRuntimeState {
    fn document_text(&mut self) -> std::result::Result<String, HostError> {
        self.require(Capability::DocumentRead)?;
        Ok(self.document_text.clone())
    }

    fn document_path(&mut self) -> std::result::Result<Option<String>, HostError> {
        self.require(Capability::DocumentRead)?;
        Ok(self.document_path.clone())
    }

    fn replace_range(
        &mut self,
        start: u64,
        end: u64,
        text: String,
    ) -> std::result::Result<(), HostError> {
        self.require(Capability::DocumentWrite)?;
        self.outputs.pending_edits.push(PendingEdit::ReplaceRange {
            start: start as usize,
            end: end as usize,
            text,
        });
        Ok(())
    }

    fn insert_text(&mut self, position: u64, text: String) -> std::result::Result<(), HostError> {
        self.require(Capability::DocumentWrite)?;
        self.outputs.pending_edits.push(PendingEdit::Insert {
            position: position as usize,
            text,
        });
        Ok(())
    }

    fn set_decorations(&mut self, data: Vec<u8>) -> std::result::Result<(), HostError> {
        self.require(Capability::Decorations)?;
        let payload: VersionedPayload<Vec<Decoration>> =
            serde_json::from_slice(&data).map_err(|err| HostError {
                message: format!("invalid decorations payload: {err}"),
            })?;
        if payload.version != 1 {
            return Err(HostError {
                message: format!(
                    "unsupported decorations payload version: {}",
                    payload.version
                ),
            });
        }
        self.outputs.decorations = Some(payload.data);
        Ok(())
    }

    fn clear_decorations(&mut self) -> std::result::Result<(), HostError> {
        self.require(Capability::Decorations)?;
        self.outputs.decorations = Some(Vec::new());
        Ok(())
    }
}

impl bindings::vellum::extension::ui::Host for ExtensionRuntimeState {
    fn set_panel_view(
        &mut self,
        panel_id: String,
        data: Vec<u8>,
    ) -> std::result::Result<(), HostError> {
        self.require(Capability::Panels)?;
        let payload: VersionedPayload<UiNode> =
            serde_json::from_slice(&data).map_err(|err| HostError {
                message: format!("invalid panel payload: {err}"),
            })?;
        if payload.version != 1 {
            return Err(HostError {
                message: format!("unsupported panel payload version: {}", payload.version),
            });
        }
        if payload.data.contains_webview() {
            self.require(Capability::Webview)?;
        }
        let qualified = self.manifest.qualified_panel_id(&panel_id);
        self.outputs.panel_uis.insert(qualified, payload.data);
        Ok(())
    }
}

impl bindings::vellum::extension::timer::Host for ExtensionRuntimeState {
    fn now_ms(&mut self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    fn request_tick(&mut self, interval_ms: u32) -> std::result::Result<(), HostError> {
        self.require(Capability::Timers)?;
        let now = self.now_ms();
        self.tick_interval_ms = Some(interval_ms);
        self.tick_next_due_ms = Some(now + interval_ms as u64);
        Ok(())
    }

    fn cancel_tick(&mut self) -> std::result::Result<(), HostError> {
        self.require(Capability::Timers)?;
        self.tick_interval_ms = None;
        self.tick_next_due_ms = None;
        Ok(())
    }
}

struct LoadedExtension {
    manifest: ExtensionManifest,
    store: Store<ExtensionRuntimeState>,
    bindings: ExtensionWorld,
}

pub struct ExtensionHost {
    engine: Engine,
    linker: Linker<ExtensionRuntimeState>,
    registry: ExtensionRegistry,
    loaded_extensions: HashMap<String, LoadedExtension>,
    commands: Vec<RegisteredCommand>,
    sidebar_panels: Vec<RegisteredPanel>,
    panel_uis: HashMap<PanelId, UiNode>,
    outputs: ExtensionOutputs,
    document_text: String,
    document_path: Option<String>,
    dev_extensions_file: PathBuf,
    #[cfg(feature = "hot-reload")]
    hot_reload_controller: Option<crate::hot_reload::HotReloadController>,
}

impl ExtensionHost {
    pub fn new() -> Result<Self> {
        let mut config = Config::new();
        config.wasm_component_model(true);
        let engine = Engine::new(&config)?;
        let mut linker = Linker::new(&engine);
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
        ExtensionWorld::add_to_linker::<
            ExtensionRuntimeState,
            wasmtime::component::HasSelf<ExtensionRuntimeState>,
        >(&mut linker, |state| state)?;

        Ok(Self {
            engine,
            linker,
            registry: ExtensionRegistry::new(),
            loaded_extensions: HashMap::new(),
            commands: Vec::new(),
            sidebar_panels: Vec::new(),
            panel_uis: HashMap::new(),
            outputs: ExtensionOutputs::default(),
            document_text: String::new(),
            document_path: None,
            dev_extensions_file: default_dev_extensions_file(),
            #[cfg(feature = "hot-reload")]
            hot_reload_controller: None,
        })
    }

    pub fn registry(&self) -> &ExtensionRegistry {
        &self.registry
    }

    pub fn registry_mut(&mut self) -> &mut ExtensionRegistry {
        &mut self.registry
    }

    pub fn discover_in_dir(&mut self, dir: &Path) -> Result<Vec<String>> {
        let discovered = self.registry.discover_in_dir(dir)?;
        self.rebuild_static_contributions();
        Ok(discovered)
    }

    pub fn load_dev_extensions(&mut self) -> Result<Vec<String>> {
        self.registry
            .load_dev_extensions_file(&self.dev_extensions_file)?;
        let discovered = self.registry.discover_dev_extensions()?;
        self.rebuild_static_contributions();
        Ok(discovered)
    }

    pub fn install_dev_extension(&mut self, dir: PathBuf) -> Result<String> {
        let id = self.registry.discover_extension_dir(&dir, true)?;
        self.registry.add_dev_extension_dir(dir);
        self.registry
            .save_dev_extensions_file(&self.dev_extensions_file)?;
        self.rebuild_static_contributions();
        Ok(id)
    }

    pub fn save_dev_extension_state(&self) -> Result<()> {
        self.registry
            .save_dev_extensions_file(&self.dev_extensions_file)
    }

    pub fn activate_discovered(&mut self) -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    pub fn unload_extension(&mut self, extension_id: &str) -> Result<()> {
        if let Some(mut loaded) = self.loaded_extensions.remove(extension_id) {
            let _ = loaded.bindings.call_deactivate(&mut loaded.store);
        }
        self.registry.disable(extension_id);
        self.panel_uis
            .retain(|panel_id, _| !panel_id.starts_with(&format!("{extension_id}.")));
        self.save_dev_extension_state()?;
        self.rebuild_static_contributions();
        Ok(())
    }

    pub fn enable_extension(&mut self, extension_id: &str) -> Result<()> {
        self.registry.enable(extension_id);
        self.save_dev_extension_state()?;
        self.rebuild_static_contributions();
        Ok(())
    }

    pub fn activate_extension(&mut self, extension_id: &str) -> Result<()> {
        if self.loaded_extensions.contains_key(extension_id) {
            return Ok(());
        }
        if self.registry.is_disabled(extension_id) {
            return Ok(());
        }

        let entry = self
            .registry
            .get(extension_id)
            .ok_or_else(|| anyhow::anyhow!("extension not found: {extension_id}"))?
            .clone();
        let component_path = entry.directory.join(&entry.manifest.wasm.component);
        if !component_path.exists() {
            anyhow::bail!(
                "extension component not found: {}",
                component_path.display()
            );
        }

        let component = Component::from_file(&self.engine, &component_path)
            .with_context(|| format!("failed to load component {}", component_path.display()))?;
        let mut store = Store::new(
            &self.engine,
            ExtensionRuntimeState::new(extension_id.to_string(), entry.manifest.clone()),
        );
        store
            .data_mut()
            .set_document(self.document_text.clone(), self.document_path.clone());
        let bindings = ExtensionWorld::instantiate(&mut store, &component, &self.linker)
            .context("failed to instantiate extension component")?;

        let ctx = ActivationContext {
            extension_id: extension_id.to_string(),
            extension_path: entry.directory.to_string_lossy().to_string(),
        };
        if let Err(err) = bindings.call_activate(&mut store, &ctx)? {
            anyhow::bail!(err.message);
        }

        self.loaded_extensions.insert(
            extension_id.to_string(),
            LoadedExtension {
                manifest: entry.manifest.clone(),
                store,
                bindings,
            },
        );
        self.registry.mark_active(extension_id);
        self.collect_extension_state(extension_id);
        Ok(())
    }

    pub fn dispatch_event(
        &mut self,
        event_type: &str,
        _document_id: &str,
        document_text: &str,
        document_path: Option<&str>,
    ) {
        self.update_document(
            document_text.to_string(),
            document_path.map(ToOwned::to_owned),
        );

        let to_activate: Vec<String> = self
            .registry
            .available_extensions()
            .into_iter()
            .filter(|entry| entry.manifest.activates_on(event_type))
            .map(|entry| entry.manifest.id.clone())
            .collect();

        for extension_id in to_activate {
            if let Err(err) = self.activate_extension(&extension_id) {
                self.registry.mark_failed(&extension_id, err.to_string());
                eprintln!("failed to activate extension {extension_id}: {err}");
            }
        }

        let event = ExtensionEvent {
            event_type: event_type.to_string(),
            document_text: document_text.to_string(),
            document_path: document_path.map(ToOwned::to_owned),
            timestamp_ms: None,
        };
        let loaded: Vec<String> = self.loaded_extensions.keys().cloned().collect();
        for extension_id in loaded {
            if self
                .loaded_extensions
                .get(&extension_id)
                .map(|loaded| loaded.manifest.activates_on(event_type))
                .unwrap_or(false)
            {
                if let Err(err) = self.call_handle_event(&extension_id, event.clone()) {
                    self.registry.mark_failed(&extension_id, err.to_string());
                    eprintln!("extension event failed for {extension_id}: {err}");
                }
            }
        }
    }

    pub fn execute_command(&mut self, qualified_command_id: &str) -> bool {
        let Some(command) = self
            .commands
            .iter()
            .find(|command| command.qualified_id == qualified_command_id)
            .cloned()
        else {
            return false;
        };

        if let Err(err) = self.activate_extension(&command.extension_id) {
            self.registry
                .mark_failed(&command.extension_id, err.to_string());
            eprintln!(
                "failed to activate extension {}: {err}",
                command.extension_id
            );
            return false;
        }

        match self.loaded_extensions.get_mut(&command.extension_id) {
            Some(loaded) => {
                let result = loaded
                    .bindings
                    .call_execute_command(&mut loaded.store, &command.qualified_id);
                if let Err(err) = result.and_then(extension_call_result) {
                    self.registry
                        .mark_failed(&command.extension_id, err.to_string());
                    eprintln!(
                        "extension command failed for {}: {err}",
                        command.extension_id
                    );
                    return false;
                }
                self.collect_extension_state(&command.extension_id);
                true
            }
            None => false,
        }
    }

    pub fn open_panel(&mut self, qualified_panel_id: &str) {
        let Some(panel) = self
            .sidebar_panels
            .iter()
            .find(|panel| panel.qualified_id == qualified_panel_id)
            .cloned()
        else {
            return;
        };

        if let Err(err) = self.activate_extension(&panel.extension_id) {
            self.registry
                .mark_failed(&panel.extension_id, err.to_string());
            eprintln!("failed to activate extension {}: {err}", panel.extension_id);
        }
    }

    pub fn handle_ui_event(&mut self, event: UiEvent) {
        let panel_id = event.panel_id().to_string();
        let Some(panel) = self
            .sidebar_panels
            .iter()
            .find(|panel| panel.qualified_id == panel_id)
            .cloned()
        else {
            return;
        };

        if let Err(err) = self.activate_extension(&panel.extension_id) {
            self.registry
                .mark_failed(&panel.extension_id, err.to_string());
            eprintln!("failed to activate extension {}: {err}", panel.extension_id);
            return;
        }

        let wit_event = ui_event_to_wit(event);
        if let Some(loaded) = self.loaded_extensions.get_mut(&panel.extension_id) {
            let result = loaded
                .bindings
                .call_handle_ui_event(&mut loaded.store, &wit_event);
            if let Err(err) = result.and_then(extension_call_result) {
                self.registry
                    .mark_failed(&panel.extension_id, err.to_string());
                eprintln!(
                    "extension UI event failed for {}: {err}",
                    panel.extension_id
                );
            }
        }
        self.collect_extension_state(&panel.extension_id);
    }

    pub fn handle_hover(&mut self, hover_data: &str) -> Option<Tooltip> {
        for extension_id in self.loaded_extensions.keys().cloned().collect::<Vec<_>>() {
            let Some(loaded) = self.loaded_extensions.get_mut(&extension_id) else {
                continue;
            };
            let result = loaded
                .bindings
                .call_handle_hover(&mut loaded.store, hover_data);
            match result {
                Ok(Ok(Some(bytes))) => {
                    if let Ok(payload) = serde_json::from_slice::<VersionedPayload<Tooltip>>(&bytes)
                    {
                        if payload.version == 1 {
                            return Some(payload.data);
                        }
                    }
                }
                Ok(Ok(None)) => {}
                Ok(Err(err)) => {
                    eprintln!("extension hover failed for {extension_id}: {}", err.message)
                }
                Err(err) => eprintln!("extension hover trapped for {extension_id}: {err}"),
            }
        }
        None
    }

    pub fn has_active_timers(&self) -> bool {
        self.loaded_extensions
            .values()
            .any(|loaded| loaded.store.data().tick_interval_ms.is_some())
    }

    pub fn dispatch_timer_ticks(&mut self, now_ms: u64) {
        let loaded: Vec<String> = self.loaded_extensions.keys().cloned().collect();
        for extension_id in loaded {
            let Some(loaded) = self.loaded_extensions.get_mut(&extension_id) else {
                continue;
            };
            let due = loaded.store.data().tick_next_due_ms;
            let interval = loaded.store.data().tick_interval_ms;
            match (due, interval) {
                (Some(due_time), Some(interval_ms)) if now_ms >= due_time => {
                    let next_due = due_time + interval_ms as u64;
                    loaded.store.data_mut().tick_next_due_ms = Some(next_due);
                }
                _ => continue,
            }

            let event = WitExtensionEvent {
                event_type: "timer.tick".to_string(),
                document_text: String::new(),
                document_path: None,
                timestamp_ms: Some(now_ms),
            };
            let result = loaded.bindings.call_handle_event(&mut loaded.store, &event);
            if let Err(err) = result.and_then(extension_call_result) {
                self.registry.mark_failed(&extension_id, err.to_string());
                eprintln!("extension timer tick failed for {extension_id}: {err}");
            }
            self.collect_extension_state(&extension_id);
        }
    }

    pub fn commands(&self) -> &[RegisteredCommand] {
        &self.commands
    }

    pub fn sidebar_panels(&self) -> &[RegisteredPanel] {
        &self.sidebar_panels
    }

    pub fn panel_ui(&self, panel_id: &str) -> Option<&UiNode> {
        self.panel_uis.get(panel_id)
    }

    pub fn loaded_manifests(&self) -> Vec<ExtensionManifest> {
        self.registry
            .all_entries()
            .into_iter()
            .map(|entry| entry.manifest.clone())
            .collect()
    }

    pub fn update_document(&mut self, text: String, path: Option<String>) {
        self.document_text = text.clone();
        self.document_path = path.clone();
        for loaded in self.loaded_extensions.values_mut() {
            loaded
                .store
                .data_mut()
                .set_document(text.clone(), path.clone());
        }
    }

    pub fn take_outputs(&mut self) -> ExtensionOutputs {
        let mut outputs = std::mem::take(&mut self.outputs);
        if !self.panel_uis.is_empty() {
            outputs.panel_uis.extend(self.panel_uis.clone());
        }
        outputs
    }

    pub fn shutdown_all(&mut self) {
        for loaded in self.loaded_extensions.values_mut() {
            let _ = loaded.bindings.call_deactivate(&mut loaded.store);
        }
        self.loaded_extensions.clear();
    }

    fn rebuild_static_contributions(&mut self) {
        self.commands.clear();
        self.sidebar_panels.clear();
        for entry in self.registry.all_entries() {
            if matches!(entry.state, ExtensionState::Disabled) {
                continue;
            }
            if entry.manifest.capabilities.commands {
                for command in &entry.manifest.contributes.commands {
                    self.commands.push(RegisteredCommand {
                        qualified_id: entry.manifest.qualified_command_id(&command.id),
                        command_id: command.id.clone(),
                        label: command.title.clone(),
                        key_binding: command.key.clone(),
                        extension_id: entry.manifest.id.clone(),
                    });
                }
            }
            if entry.manifest.capabilities.panels {
                for panel in &entry.manifest.contributes.panels {
                    self.sidebar_panels.push(RegisteredPanel {
                        qualified_id: entry.manifest.qualified_panel_id(&panel.id),
                        panel_id: panel.id.clone(),
                        label: panel.title.clone(),
                        icon: panel.icon.clone(),
                        extension_id: entry.manifest.id.clone(),
                    });
                }
            }
        }
    }

    fn call_handle_event(&mut self, extension_id: &str, event: ExtensionEvent) -> Result<()> {
        let Some(loaded) = self.loaded_extensions.get_mut(extension_id) else {
            return Ok(());
        };
        let event = WitExtensionEvent {
            event_type: event.event_type,
            document_text: event.document_text,
            document_path: event.document_path,
            timestamp_ms: event.timestamp_ms,
        };
        let result = loaded.bindings.call_handle_event(&mut loaded.store, &event);
        result.and_then(extension_call_result)?;
        self.collect_extension_state(extension_id);
        Ok(())
    }

    fn collect_extension_state(&mut self, extension_id: &str) {
        let Some(loaded) = self.loaded_extensions.get_mut(extension_id) else {
            return;
        };
        let outputs = loaded.store.data_mut().take_outputs();
        if !outputs.panel_uis.is_empty() {
            self.panel_uis.extend(outputs.panel_uis.clone());
        }
        self.outputs.merge(outputs);
    }

    pub fn is_extension_loaded(&self, extension_id: &str) -> bool {
        self.loaded_extensions.contains_key(extension_id)
    }

    pub fn panel_uis(&self) -> &HashMap<PanelId, UiNode> {
        &self.panel_uis
    }

    pub fn set_panel_view(&mut self, panel_id: PanelId, ui_node: UiNode) {
        self.panel_uis.insert(panel_id, ui_node);
    }

    #[cfg(feature = "hot-reload")]
    pub fn init_hot_reload(&mut self) {
        self.hot_reload_controller = Some(crate::hot_reload::HotReloadController::new());
    }

    #[cfg(feature = "hot-reload")]
    pub fn hot_reload_controller(&self) -> Option<&crate::hot_reload::HotReloadController> {
        self.hot_reload_controller.as_ref()
    }

    #[cfg(feature = "hot-reload")]
    pub fn hot_reload_controller_mut(&mut self) -> Option<&mut crate::hot_reload::HotReloadController> {
        self.hot_reload_controller.as_mut()
    }
}

fn extension_call_result(result: std::result::Result<(), ExtensionError>) -> wasmtime::Result<()> {
    match result {
        Ok(()) => Ok(()),
        Err(err) => anyhow::bail!(err.message),
    }
}

fn ui_event_to_wit(event: UiEvent) -> WitUiEvent {
    match event {
        UiEvent::ButtonClicked {
            panel_id,
            element_id,
        } => WitUiEvent {
            panel_id,
            element_id,
            event_kind: "button.clicked".into(),
            value: None,
            index: None,
            checked: None,
        },
        UiEvent::InputChanged {
            panel_id,
            element_id,
            value,
        } => WitUiEvent {
            panel_id,
            element_id,
            event_kind: "input.changed".into(),
            value: Some(value),
            index: None,
            checked: None,
        },
        UiEvent::CheckboxToggled {
            panel_id,
            element_id,
            checked,
        } => WitUiEvent {
            panel_id,
            element_id,
            event_kind: "checkbox.toggled".into(),
            value: None,
            index: None,
            checked: Some(checked),
        },
        UiEvent::SelectChanged {
            panel_id,
            element_id,
            index,
        } => WitUiEvent {
            panel_id,
            element_id,
            event_kind: "select.changed".into(),
            value: None,
            index: Some(index as u32),
            checked: None,
        },
        UiEvent::ToggleChanged {
            panel_id,
            element_id,
            active,
        } => WitUiEvent {
            panel_id,
            element_id,
            event_kind: "toggle.changed".into(),
            value: None,
            index: None,
            checked: Some(active),
        },
        UiEvent::LinkClicked {
            panel_id,
            element_id,
        } => WitUiEvent {
            panel_id,
            element_id,
            event_kind: "link.clicked".into(),
            value: None,
            index: None,
            checked: None,
        },
        UiEvent::ListItemClicked {
            panel_id,
            element_id,
            item_id,
        } => WitUiEvent {
            panel_id,
            element_id,
            event_kind: "list.item.clicked".into(),
            value: Some(item_id),
            index: None,
            checked: None,
        },
        UiEvent::DisclosureToggled {
            panel_id,
            element_id,
            open,
        } => WitUiEvent {
            panel_id,
            element_id,
            event_kind: "disclosure.toggled".into(),
            value: None,
            index: None,
            checked: Some(open),
        },
    }
}

fn default_dev_extensions_file() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".vellum")
        .join("dev-extensions.toml")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{Capabilities, ExtensionManifest, WasmConfig};

    #[test]
    fn host_initializes() {
        assert!(ExtensionHost::new().is_ok());
    }

    #[test]
    fn panel_view_rejects_webview_without_capability() {
        let manifest = ExtensionManifest {
            id: "test.webview".into(),
            name: "WebView Test".into(),
            version: "0.1.0".into(),
            schema_version: 1,
            authors: Vec::new(),
            description: String::new(),
            repository: String::new(),
            wasm: WasmConfig {
                component: "target/wasm32-wasip2/release/test.wasm".into(),
            },
            activation: Default::default(),
            capabilities: Capabilities {
                panels: true,
                webview: false,
                ..Default::default()
            },
            contributes: Default::default(),
        };
        let mut state = ExtensionRuntimeState::new("test.webview".into(), manifest);
        let payload = VersionedPayload::new(UiNode::WebView {
            id: "preview".into(),
            url: "https://example.com".into(),
            allow_scripts: false,
            allow_devtools: false,
        });
        let bytes = serde_json::to_vec(&payload).unwrap();

        let err = <ExtensionRuntimeState as bindings::vellum::extension::ui::Host>::set_panel_view(
            &mut state,
            "panel".into(),
            bytes,
        )
        .unwrap_err();

        assert!(err.message.contains("webview"));
    }

    #[test]
    fn manifest_parses_timers_capability() {
        let toml = r#"
id = "test.timer"
name = "Timer Test"
version = "0.1.0"
schema_version = 1

[wasm]
component = "target/wasm32-wasip2/release/test.wasm"

[capabilities]
timers = true
"#;
        let manifest = ExtensionManifest::from_toml_str(toml).unwrap();
        assert!(manifest.capabilities.timers);
    }

    #[test]
    fn request_tick_rejected_without_timers_capability() {
        let manifest = ExtensionManifest {
            id: "test.no-timer".into(),
            name: "No Timer".into(),
            version: "0.1.0".into(),
            schema_version: 1,
            authors: Vec::new(),
            description: String::new(),
            repository: String::new(),
            wasm: WasmConfig {
                component: "target/wasm32-wasip2/release/test.wasm".into(),
            },
            activation: Default::default(),
            capabilities: Capabilities {
                timers: false,
                ..Default::default()
            },
            contributes: Default::default(),
        };
        let mut state = ExtensionRuntimeState::new("test.no-timer".into(), manifest);

        let err =
            <ExtensionRuntimeState as bindings::vellum::extension::timer::Host>::request_tick(
                &mut state, 1000,
            )
            .unwrap_err();
        assert!(err.message.contains("timers"));
    }

    #[test]
    fn request_tick_sets_interval_and_due_time() {
        let manifest = ExtensionManifest {
            id: "test.timer".into(),
            name: "Timer Test".into(),
            version: "0.1.0".into(),
            schema_version: 1,
            authors: Vec::new(),
            description: String::new(),
            repository: String::new(),
            wasm: WasmConfig {
                component: "target/wasm32-wasip2/release/test.wasm".into(),
            },
            activation: Default::default(),
            capabilities: Capabilities {
                timers: true,
                ..Default::default()
            },
            contributes: Default::default(),
        };
        let mut state = ExtensionRuntimeState::new("test.timer".into(), manifest);

        <ExtensionRuntimeState as bindings::vellum::extension::timer::Host>::request_tick(
            &mut state, 1000,
        )
        .unwrap();

        assert_eq!(state.tick_interval_ms, Some(1000));
        assert!(state.tick_next_due_ms.is_some());
    }

    #[test]
    fn cancel_tick_clears_interval_and_due_time() {
        let manifest = ExtensionManifest {
            id: "test.timer".into(),
            name: "Timer Test".into(),
            version: "0.1.0".into(),
            schema_version: 1,
            authors: Vec::new(),
            description: String::new(),
            repository: String::new(),
            wasm: WasmConfig {
                component: "target/wasm32-wasip2/release/test.wasm".into(),
            },
            activation: Default::default(),
            capabilities: Capabilities {
                timers: true,
                ..Default::default()
            },
            contributes: Default::default(),
        };
        let mut state = ExtensionRuntimeState::new("test.timer".into(), manifest);

        <ExtensionRuntimeState as bindings::vellum::extension::timer::Host>::request_tick(
            &mut state, 1000,
        )
        .unwrap();
        <ExtensionRuntimeState as bindings::vellum::extension::timer::Host>::cancel_tick(
            &mut state,
        )
        .unwrap();

        assert_eq!(state.tick_interval_ms, None);
        assert_eq!(state.tick_next_due_ms, None);
    }
}
