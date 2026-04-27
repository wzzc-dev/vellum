use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use wasmi::{Engine, Instance, Linker, Memory, Module, Store};

use crate::contributions::{
    Decoration, OverlayPanel, PendingEdit, ProtocolResponse, RegisteredCommand, RegisteredPanel,
    Tooltip, WebViewRequest,
};
use crate::manifest::ExtensionManifest;
use crate::permissions::PermissionChecker;
use crate::registry::{ExtensionRegistry, ExtensionState};
use crate::ui::{UiEvent, UiNode};

// ── Per-extension host state (wasmi host state) ───────────────

pub struct ExtensionHostState {
    pub extension_id: String,
    pub extension_path: PathBuf,
    pub manifest: ExtensionManifest,

    // Memory allocator
    pub alloc_offset: u32,

    // Pending contributions
    pub pending_commands: Vec<RegisteredCommand>,
    pub pending_panels: Vec<RegisteredPanel>,
    pub pending_subscriptions: Vec<(u32, u32)>,
    pub next_command_id: u32,
    pub next_panel_id: u32,
    pub next_subscription_id: u32,

    // Document state
    pub document_text: String,
    pub document_path: Option<String>,

    // UI state
    pub panel_uis: HashMap<u32, UiNode>,
    pub decorations: Vec<Decoration>,
    pub active_overlay: Option<OverlayPanel>,
    pub active_tooltip: Option<Tooltip>,
    pub status_message: Option<String>,

    // Edit queue
    pub pending_edits: Vec<PendingEdit>,

    // WebView state
    pub pending_webview_requests: Vec<WebViewRequest>,
    pub next_webview_id: u32,
    pub protocol_responses: HashMap<u32, ProtocolResponse>,
}

impl ExtensionHostState {
    pub fn new(
        extension_id: String,
        extension_path: PathBuf,
        manifest: ExtensionManifest,
    ) -> Self {
        Self {
            extension_id,
            extension_path,
            manifest,
            alloc_offset: 65536,
            pending_commands: Vec::new(),
            pending_panels: Vec::new(),
            pending_subscriptions: Vec::new(),
            next_command_id: 1,
            next_panel_id: 1,
            next_subscription_id: 1,
            document_text: String::new(),
            document_path: None,
            panel_uis: HashMap::new(),
            decorations: Vec::new(),
            active_overlay: None,
            active_tooltip: None,
            status_message: None,
            pending_edits: Vec::new(),
            pending_webview_requests: Vec::new(),
            next_webview_id: 1,
            protocol_responses: HashMap::new(),
        }
    }

    pub fn update_document(&mut self, text: String, path: Option<String>) {
        self.document_text = text;
        self.document_path = path;
    }

    pub fn take_commands(&mut self) -> Vec<RegisteredCommand> {
        std::mem::take(&mut self.pending_commands)
    }

    pub fn take_panels(&mut self) -> Vec<RegisteredPanel> {
        std::mem::take(&mut self.pending_panels)
    }

    pub fn take_subscriptions(&mut self) -> Vec<(u32, u32)> {
        std::mem::take(&mut self.pending_subscriptions)
    }

    pub fn take_status_message(&mut self) -> Option<String> {
        std::mem::take(&mut self.status_message)
    }

    pub fn take_edits(&mut self) -> Vec<PendingEdit> {
        std::mem::take(&mut self.pending_edits)
    }

    pub fn take_webview_requests(&mut self) -> Vec<WebViewRequest> {
        std::mem::take(&mut self.pending_webview_requests)
    }
}

// ── Loaded extension ──────────────────────────────────────────

struct LoadedExtension {
    manifest: ExtensionManifest,
    instance: Instance,
    store: Store<ExtensionHostState>,
    memory: Memory,
    subscribed_events: Vec<u32>,
}

// ── Extension Host ────────────────────────────────────────────

pub struct ExtensionHost {
    engine: Engine,
    linker: Linker<ExtensionHostState>,
    registry: ExtensionRegistry,
    loaded_extensions: Vec<LoadedExtension>,

    // Aggregated contributions
    commands: Vec<RegisteredCommand>,
    sidebar_panels: Vec<RegisteredPanel>,
    panel_uis: HashMap<u32, UiNode>,
    decorations: Vec<Decoration>,
    active_overlay: Option<OverlayPanel>,
    active_tooltip: Option<Tooltip>,
    pending_status_message: Option<String>,
    pending_edits: Vec<PendingEdit>,
    webview_requests: Vec<WebViewRequest>,
}

impl ExtensionHost {
    pub fn new() -> Result<Self> {
        let engine = Engine::default();
        let mut linker = <Linker<ExtensionHostState>>::new(&engine);

        // Register host functions
        Self::register_host_functions(&mut linker)?;

        Ok(Self {
            engine,
            linker,
            registry: ExtensionRegistry::new(),
            loaded_extensions: Vec::new(),
            commands: Vec::new(),
            sidebar_panels: Vec::new(),
            panel_uis: HashMap::new(),
            decorations: Vec::new(),
            active_overlay: None,
            active_tooltip: None,
            pending_status_message: None,
            pending_edits: Vec::new(),
            webview_requests: Vec::new(),
        })
    }

    fn register_host_functions(linker: &mut Linker<ExtensionHostState>) -> Result<()> {
        linker
            .func_wrap("env", "host_alloc", host_alloc_impl)
            .context("failed to define host_alloc")?;
        linker
            .func_wrap("env", "host_dealloc", host_dealloc_impl)
            .context("failed to define host_dealloc")?;
        linker
            .func_wrap("env", "host_register_command", host_register_command_impl)
            .context("failed to define host_register_command")?;
        linker
            .func_wrap(
                "env",
                "host_register_sidebar_panel",
                host_register_sidebar_panel_impl,
            )
            .context("failed to define host_register_sidebar_panel")?;
        linker
            .func_wrap("env", "host_subscribe_event", host_subscribe_event_impl)
            .context("failed to define host_subscribe_event")?;
        linker
            .func_wrap(
                "env",
                "host_set_status_message",
                host_set_status_message_impl,
            )
            .context("failed to define host_set_status_message")?;
        linker
            .func_wrap(
                "env",
                "host_get_document_text",
                host_get_document_text_impl,
            )
            .context("failed to define host_get_document_text")?;
        linker
            .func_wrap(
                "env",
                "host_get_document_path",
                host_get_document_path_impl,
            )
            .context("failed to define host_get_document_path")?;
        linker
            .func_wrap("env", "host_set_panel_ui", host_set_panel_ui_impl)
            .context("failed to define host_set_panel_ui")?;
        linker
            .func_wrap(
                "env",
                "host_set_decorations",
                host_set_decorations_impl,
            )
            .context("failed to define host_set_decorations")?;
        linker
            .func_wrap(
                "env",
                "host_clear_decorations",
                host_clear_decorations_impl,
            )
            .context("failed to define host_clear_decorations")?;
        linker
            .func_wrap("env", "host_show_overlay", host_show_overlay_impl)
            .context("failed to define host_show_overlay")?;
        linker
            .func_wrap("env", "host_hide_overlay", host_hide_overlay_impl)
            .context("failed to define host_hide_overlay")?;
        linker
            .func_wrap("env", "host_show_tooltip", host_show_tooltip_impl)
            .context("failed to define host_show_tooltip")?;
        linker
            .func_wrap("env", "host_hide_tooltip", host_hide_tooltip_impl)
            .context("failed to define host_hide_tooltip")?;
        linker
            .func_wrap("env", "host_insert_text", host_insert_text_impl)
            .context("failed to define host_insert_text")?;
        linker
            .func_wrap("env", "host_replace_range", host_replace_range_impl)
            .context("failed to define host_replace_range")?;
        linker
            .func_wrap("env", "host_create_webview", host_create_webview_impl)
            .context("failed to define host_create_webview")?;
        linker
            .func_wrap(
                "env",
                "host_navigate_webview",
                host_navigate_webview_impl,
            )
            .context("failed to define host_navigate_webview")?;
        linker
            .func_wrap(
                "env",
                "host_respond_webview_request",
                host_respond_webview_request_impl,
            )
            .context("failed to define host_respond_webview_request")?;
        Ok(())
    }

    // ── Registry access ───────────────────────────────────────

    pub fn registry(&self) -> &ExtensionRegistry {
        &self.registry
    }

    pub fn registry_mut(&mut self) -> &mut ExtensionRegistry {
        &mut self.registry
    }

    // ── Discovery ─────────────────────────────────────────────

    pub fn discover_in_dir(&mut self, dir: &Path) -> Result<Vec<String>> {
        self.registry.discover_in_dir(dir)
    }

    pub fn activate_discovered(&mut self) -> Result<Vec<String>> {
        let to_activate: Vec<String> = self
            .registry
            .discovered_extensions()
            .iter()
            .map(|e| e.manifest.id.clone())
            .collect();

        let mut activated = Vec::new();
        for ext_id in to_activate {
            match self.load_and_activate(&ext_id) {
                Ok(()) => activated.push(ext_id),
                Err(e) => {
                    eprintln!("failed to activate extension {}: {}", ext_id, e);
                    self.registry.mark_failed(&ext_id, format!("{}", e));
                }
            }
        }
        Ok(activated)
    }

    fn load_and_activate(&mut self, extension_id: &str) -> Result<()> {
        let entry = self
            .registry
            .get(extension_id)
            .ok_or_else(|| anyhow::anyhow!("extension not found: {}", extension_id))?
            .clone();

        let wasm_path = entry.directory.join(&entry.manifest.entry);
        if !wasm_path.exists() {
            anyhow::bail!("extension entry file not found: {}", wasm_path.display());
        }

        let wasm_bytes = std::fs::read(&wasm_path)
            .with_context(|| format!("failed to read WASM: {}", wasm_path.display()))?;

        let module = Module::new(&self.engine, &wasm_bytes)
            .context("failed to compile WASM module")?;

        let state = ExtensionHostState::new(
            extension_id.to_string(),
            entry.directory.clone(),
            entry.manifest.clone(),
        );
        let mut store = Store::new(&self.engine, state);

        let instance_pre = self
            .linker
            .instantiate(&mut store, &module)
            .context("failed to instantiate WASM module")?;

        let instance = instance_pre
            .start(&mut store)
            .context("failed to start WASM module")?;

        let memory = get_memory(&instance, &store)?;

        // Call plugin_init
        Self::call_init(&instance, &mut store)?;

        // Collect contributions
        let new_commands = store.data_mut().take_commands();
        let new_panels = store.data_mut().take_panels();
        let new_subscriptions = store.data_mut().take_subscriptions();

        let subscribed_events: Vec<u32> = new_subscriptions
            .iter()
            .map(|(event_type, _)| *event_type)
            .collect();

        self.commands.extend(new_commands);
        self.sidebar_panels.extend(new_panels);

        let loaded = LoadedExtension {
            manifest: entry.manifest.clone(),
            instance,
            store,
            memory,
            subscribed_events,
        };
        self.loaded_extensions.push(loaded);
        self.registry.mark_active(extension_id);

        Ok(())
    }

    // ── Unload ────────────────────────────────────────────────

    pub fn unload_extension(&mut self, extension_id: &str) -> Result<()> {
        if let Some(pos) = self
            .loaded_extensions
            .iter()
            .position(|p| p.manifest.id == extension_id)
        {
            let mut plugin = self.loaded_extensions.remove(pos);
            Self::call_shutdown(&mut plugin);
            self.commands.retain(|c| c.extension_id != extension_id);
            self.sidebar_panels
                .retain(|p| p.extension_id != extension_id);
        }
        self.registry.disable(extension_id);
        Ok(())
    }

    // ── Event dispatch ────────────────────────────────────────

    pub fn dispatch_event(
        &mut self,
        event_type: &str,
        document_id: &str,
        document_text: &str,
        document_path: Option<&str>,
    ) {
        // Map string event type to u32 code matching SDK EventType enum
        let event_type_code: u32 = match event_type {
            "document.opened" => 0,
            "document.closed" => 1,
            "document.changed" => 2,
            "document.saved" => 3,
            "selection.changed" => 4,
            "editor.focused" => 5,
            "editor.blurred" => 6,
            _ => 2, // default to DocumentChanged
        };

        // Serialize event data using the EventData enum for postcard compatibility
        let event_data = crate::event::EventData::DocumentChanged {
            text: document_text.to_string(),
            path: document_path.map(|s| s.to_string()),
        };
        let event_bytes = postcard::to_allocvec(&event_data).unwrap_or_default();

        for i in 0..self.loaded_extensions.len() {
            let plugin = &mut self.loaded_extensions[i];
            plugin.store.data_mut().update_document(
                document_text.to_string(),
                document_path.map(|s| s.to_string()),
            );

            // plugin_handle_event expects (event_type: u32, data_ptr: u32, data_len: u32)
            if let Ok(func) = plugin.instance.get_typed_func::<(u32, u32, u32), ()>(
                &plugin.store,
                "plugin_handle_event",
            ) {
                let data_ptr = Self::write_to_plugin_memory(plugin, &event_bytes);
                let _ = func.call(
                    &mut plugin.store,
                    (event_type_code, data_ptr, event_bytes.len() as u32),
                );
            }

            let plugin = &mut self.loaded_extensions[i];
            Self::collect_plugin_state(
                plugin,
                &mut self.panel_uis,
                &mut self.decorations,
                &mut self.active_overlay,
                &mut self.active_tooltip,
                &mut self.pending_status_message,
                &mut self.pending_edits,
                &mut self.webview_requests,
            );
        }
    }

    pub fn execute_command(&mut self, command_id: u32) -> bool {
        for i in 0..self.loaded_extensions.len() {
            let plugin = &mut self.loaded_extensions[i];
            if let Ok(func) = plugin
                .instance
                .get_typed_func::<u32, ()>(&plugin.store, "plugin_execute_command")
            {
                let _ = func.call(&mut plugin.store, command_id);
                Self::collect_plugin_state(
                    plugin,
                    &mut self.panel_uis,
                    &mut self.decorations,
                    &mut self.active_overlay,
                    &mut self.active_tooltip,
                    &mut self.pending_status_message,
                    &mut self.pending_edits,
                    &mut self.webview_requests,
                );
                return true;
            }
        }
        false
    }

    pub fn handle_ui_event(&mut self, event: UiEvent) {
        let encoded = postcard::to_allocvec(&event).unwrap_or_default();

        for i in 0..self.loaded_extensions.len() {
            let plugin = &mut self.loaded_extensions[i];
            if let Ok(func) = plugin.instance.get_typed_func::<(u32, u32), ()>(
                &plugin.store,
                "plugin_handle_ui_event",
            ) {
                let data_ptr = Self::write_to_plugin_memory(plugin, &encoded);
                let _ = func.call(&mut plugin.store, (data_ptr, encoded.len() as u32));
            }

            let plugin = &mut self.loaded_extensions[i];
            Self::collect_plugin_state(
                plugin,
                &mut self.panel_uis,
                &mut self.decorations,
                &mut self.active_overlay,
                &mut self.active_tooltip,
                &mut self.pending_status_message,
                &mut self.pending_edits,
                &mut self.webview_requests,
            );
        }
    }

    pub fn handle_hover(&mut self, hover_data: &str) -> Option<Tooltip> {
        let hover_bytes = hover_data.as_bytes();

        for i in 0..self.loaded_extensions.len() {
            let plugin = &mut self.loaded_extensions[i];
            if let Ok(func) = plugin.instance.get_typed_func::<(u32, u32), u64>(
                &plugin.store,
                "plugin_handle_hover",
            ) {
                let data_ptr = Self::write_to_plugin_memory(plugin, hover_bytes);
                let result = func.call(&mut plugin.store, (data_ptr, hover_bytes.len() as u32));
                if let Ok(packed) = result {
                    if packed != 0 {
                        let ptr = (packed >> 32) as u32;
                        let len = (packed & 0xFFFFFFFF) as u32;
                        let bytes = read_memory(&plugin.memory, &plugin.store, ptr, len);
                        if let Ok(tooltip) = postcard::from_bytes::<Option<Tooltip>>(&bytes) {
                            if let Some(t) = tooltip {
                                return Some(t);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    pub fn dispatch_webview_request(&mut self, request: WebViewRequest) {
        let encoded = postcard::to_allocvec(&request).unwrap_or_default();

        for i in 0..self.loaded_extensions.len() {
            let plugin = &mut self.loaded_extensions[i];
            if let Ok(func) = plugin.instance.get_typed_func::<(u32, u32), ()>(
                &plugin.store,
                "plugin_handle_webview_request",
            ) {
                let data_ptr = Self::write_to_plugin_memory(plugin, &encoded);
                let _ = func.call(&mut plugin.store, (data_ptr, encoded.len() as u32));
            }

            let plugin = &mut self.loaded_extensions[i];
            Self::collect_plugin_state(
                plugin,
                &mut self.panel_uis,
                &mut self.decorations,
                &mut self.active_overlay,
                &mut self.active_tooltip,
                &mut self.pending_status_message,
                &mut self.pending_edits,
                &mut self.webview_requests,
            );
        }
    }

    // ── Accessors ─────────────────────────────────────────────

    pub fn commands(&self) -> &[RegisteredCommand] {
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

    pub fn take_webview_requests(&mut self) -> Vec<WebViewRequest> {
        std::mem::take(&mut self.webview_requests)
    }

    pub fn loaded_manifests(&self) -> Vec<ExtensionManifest> {
        self.loaded_extensions
            .iter()
            .map(|e| e.manifest.clone())
            .collect()
    }

    pub fn update_document(&mut self, text: String, path: Option<String>) {
        for plugin in &mut self.loaded_extensions {
            plugin
                .store
                .data_mut()
                .update_document(text.clone(), path.clone());
        }
    }

    pub fn shutdown_all(&mut self) {
        for plugin in &mut self.loaded_extensions {
            Self::call_shutdown(plugin);
        }
        self.loaded_extensions.clear();
    }

    // ── Internal helpers ──────────────────────────────────────

    fn call_init(instance: &Instance, store: &mut Store<ExtensionHostState>) -> Result<()> {
        let func = instance
            .get_typed_func::<(), ()>(&*store, "plugin_init")
            .context("WASM module does not export 'plugin_init'")?;
        func.call(&mut *store, ())
            .context("failed to call plugin_init")
    }

    fn call_shutdown(plugin: &mut LoadedExtension) {
        if let Ok(func) = plugin
            .instance
            .get_typed_func::<(), ()>(&plugin.store, "plugin_shutdown")
        {
            let _ = func.call(&mut plugin.store, ());
        }
    }

    fn write_to_plugin_memory(plugin: &mut LoadedExtension, data: &[u8]) -> u32 {
        let ptr = plugin.store.data_mut().alloc_offset;
        plugin.store.data_mut().alloc_offset += data.len() as u32;

        if write_memory(&plugin.memory, &mut plugin.store, ptr, data).is_ok() {
            ptr
        } else {
            0
        }
    }

    fn collect_plugin_state(
        plugin: &mut LoadedExtension,
        panel_uis: &mut HashMap<u32, UiNode>,
        decorations: &mut Vec<Decoration>,
        active_overlay: &mut Option<OverlayPanel>,
        active_tooltip: &mut Option<Tooltip>,
        status_message: &mut Option<String>,
        pending_edits: &mut Vec<PendingEdit>,
        webview_requests: &mut Vec<WebViewRequest>,
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

        let requests = state.take_webview_requests();
        if !requests.is_empty() {
            webview_requests.extend(requests);
        }
    }
}

// ── Memory helpers ────────────────────────────────────────────

fn get_memory(
    instance: &wasmi::Instance,
    store: &Store<ExtensionHostState>,
) -> Result<Memory> {
    instance
        .get_export(&store, "memory")
        .and_then(|e| e.into_memory())
        .ok_or_else(|| anyhow::anyhow!("WASM module does not export 'memory'"))
}

fn read_memory(memory: &Memory, store: &Store<ExtensionHostState>, ptr: u32, len: u32) -> Vec<u8> {
    let data = memory.data(&store);
    let start = ptr as usize;
    let end = start + len as usize;
    if end <= data.len() {
        data[start..end].to_vec()
    } else {
        Vec::new()
    }
}

fn write_memory(
    memory: &Memory,
    store: &mut Store<ExtensionHostState>,
    ptr: u32,
    data: &[u8],
) -> Result<()> {
    let mem_data = memory.data_mut(&mut *store);
    let start = ptr as usize;
    let end = start + data.len();
    if end <= mem_data.len() {
        mem_data[start..end].copy_from_slice(data);
        Ok(())
    } else {
        anyhow::bail!("write out of bounds: {}..{} > {}", start, end, mem_data.len())
    }
}

fn read_string_from_caller(
    caller: &wasmi::Caller<ExtensionHostState>,
    ptr: u32,
    len: u32,
) -> String {
    let memory = match caller.get_export("memory") {
        Some(wasmi::Extern::Memory(mem)) => mem,
        _ => return String::new(),
    };
    let data = memory.data(caller);
    let start = ptr as usize;
    let end = start + len as usize;
    if end <= data.len() {
        String::from_utf8_lossy(&data[start..end]).into_owned()
    } else {
        String::new()
    }
}

// ── Host function implementations ─────────────────────────────

fn host_alloc_impl(mut caller: wasmi::Caller<ExtensionHostState>, size: u32) -> u32 {
    let old_offset = caller.data_mut().alloc_offset;
    caller.data_mut().alloc_offset = old_offset + size;
    old_offset
}

fn host_dealloc_impl(_caller: wasmi::Caller<ExtensionHostState>, _ptr: u32, _size: u32) {}

fn host_register_command_impl(
    mut caller: wasmi::Caller<ExtensionHostState>,
    id_ptr: u32,
    id_len: u32,
    label_ptr: u32,
    label_len: u32,
    key_ptr: u32,
    key_len: u32,
) -> u32 {
    let id = read_string_from_caller(&caller, id_ptr, id_len);
    let label = read_string_from_caller(&caller, label_ptr, label_len);
    let key_binding = if key_len > 0 {
        Some(read_string_from_caller(&caller, key_ptr, key_len))
    } else {
        None
    };

    let data = caller.data_mut();
    let cmd_id = data.next_command_id;
    data.next_command_id += 1;
    data.pending_commands.push(RegisteredCommand {
        id: cmd_id,
        command_id: id,
        label,
        key_binding,
        extension_id: data.extension_id.clone(),
    });
    cmd_id
}

fn host_register_sidebar_panel_impl(
    mut caller: wasmi::Caller<ExtensionHostState>,
    id_ptr: u32,
    id_len: u32,
    label_ptr: u32,
    label_len: u32,
    icon: u32,
) -> u32 {
    let id = read_string_from_caller(&caller, id_ptr, id_len);
    let label = read_string_from_caller(&caller, label_ptr, label_len);
    let icon_name = match icon {
        0 => "file-text",
        1 => "search",
        2 => "triangle-alert",
        3 => "settings",
        4 => "bar-chart",
        _ => "file-text",
    };

    let data = caller.data_mut();
    let panel_id = data.next_panel_id;
    data.next_panel_id += 1;
    data.pending_panels.push(RegisteredPanel {
        id: panel_id,
        panel_id: id,
        label,
        icon: icon_name.into(),
        extension_id: data.extension_id.clone(),
    });
    panel_id
}

fn host_subscribe_event_impl(
    mut caller: wasmi::Caller<ExtensionHostState>,
    event_type: u32,
) -> u32 {
    let data = caller.data_mut();
    let sub_id = data.next_subscription_id;
    data.next_subscription_id += 1;
    data.pending_subscriptions.push((event_type, sub_id));
    sub_id
}

fn host_set_status_message_impl(
    mut caller: wasmi::Caller<ExtensionHostState>,
    msg_ptr: u32,
    msg_len: u32,
) {
    let msg = read_string_from_caller(&caller, msg_ptr, msg_len);
    caller.data_mut().status_message = Some(msg);
}

fn host_get_document_text_impl(
    mut caller: wasmi::Caller<ExtensionHostState>,
    buf_ptr: u32,
    buf_len: u32,
) -> u32 {
    let text_bytes = caller.data().document_text.as_bytes().to_vec();
    let write_len = (text_bytes.len() as u32).min(buf_len) as usize;
    if write_len > 0 {
        let memory = match caller.get_export("memory") {
            Some(wasmi::Extern::Memory(mem)) => mem,
            _ => return 0,
        };
        let start = buf_ptr as usize;
        if let Some(slice) = memory.data_mut(&mut caller).get_mut(start..start + write_len) {
            slice.copy_from_slice(&text_bytes[..write_len]);
        }
    }
    write_len as u32
}

fn host_get_document_path_impl(
    mut caller: wasmi::Caller<ExtensionHostState>,
    buf_ptr: u32,
    buf_len: u32,
) -> u32 {
    let path_bytes = match &caller.data().document_path {
        Some(p) => p.as_bytes().to_vec(),
        None => return 0,
    };
    let write_len = (path_bytes.len() as u32).min(buf_len) as usize;
    if write_len > 0 {
        let memory = match caller.get_export("memory") {
            Some(wasmi::Extern::Memory(mem)) => mem,
            _ => return 0,
        };
        let start = buf_ptr as usize;
        if let Some(slice) = memory.data_mut(&mut caller).get_mut(start..start + write_len) {
            slice.copy_from_slice(&path_bytes[..write_len]);
        }
    }
    write_len as u32
}

fn host_set_panel_ui_impl(
    mut caller: wasmi::Caller<ExtensionHostState>,
    panel_id: u32,
    ui_ptr: u32,
    ui_len: u32,
) {
    let bytes = {
        let memory = match caller.get_export("memory") {
            Some(wasmi::Extern::Memory(mem)) => mem,
            _ => return,
        };
        let data = memory.data(&caller);
        let start = ui_ptr as usize;
        let end = start + ui_len as usize;
        if end <= data.len() {
            data[start..end].to_vec()
        } else {
            return;
        }
    };
    if let Ok(ui) = postcard::from_bytes::<UiNode>(&bytes) {
        caller.data_mut().panel_uis.insert(panel_id, ui);
    }
}

fn host_set_decorations_impl(
    mut caller: wasmi::Caller<ExtensionHostState>,
    decos_ptr: u32,
    decos_len: u32,
) {
    let bytes = {
        let memory = match caller.get_export("memory") {
            Some(wasmi::Extern::Memory(mem)) => mem,
            _ => return,
        };
        let data = memory.data(&caller);
        let start = decos_ptr as usize;
        let end = start + decos_len as usize;
        if end <= data.len() {
            data[start..end].to_vec()
        } else {
            return;
        }
    };
    if let Ok(decos) = postcard::from_bytes::<Vec<Decoration>>(&bytes) {
        caller.data_mut().decorations = decos;
    }
}

fn host_clear_decorations_impl(mut caller: wasmi::Caller<ExtensionHostState>) {
    caller.data_mut().decorations.clear();
}

fn host_show_overlay_impl(
    mut caller: wasmi::Caller<ExtensionHostState>,
    overlay_ptr: u32,
    overlay_len: u32,
) {
    let bytes = {
        let memory = match caller.get_export("memory") {
            Some(wasmi::Extern::Memory(mem)) => mem,
            _ => return,
        };
        let data = memory.data(&caller);
        let start = overlay_ptr as usize;
        let end = start + overlay_len as usize;
        if end <= data.len() {
            data[start..end].to_vec()
        } else {
            return;
        }
    };
    if let Ok(overlay) = postcard::from_bytes::<OverlayPanel>(&bytes) {
        caller.data_mut().active_overlay = Some(overlay);
    }
}

fn host_hide_overlay_impl(
    mut caller: wasmi::Caller<ExtensionHostState>,
    _id_ptr: u32,
    _id_len: u32,
) {
    caller.data_mut().active_overlay = None;
}

fn host_show_tooltip_impl(
    mut caller: wasmi::Caller<ExtensionHostState>,
    _position: u32,
    content_ptr: u32,
    content_len: u32,
) {
    let bytes = {
        let memory = match caller.get_export("memory") {
            Some(wasmi::Extern::Memory(mem)) => mem,
            _ => return,
        };
        let data = memory.data(&caller);
        let start = content_ptr as usize;
        let end = start + content_len as usize;
        if end <= data.len() {
            data[start..end].to_vec()
        } else {
            return;
        }
    };
    if let Ok(content) = postcard::from_bytes::<UiNode>(&bytes) {
        caller.data_mut().active_tooltip = Some(Tooltip {
            content,
            position: crate::contributions::TooltipPosition::Above,
        });
    }
}

fn host_hide_tooltip_impl(mut caller: wasmi::Caller<ExtensionHostState>) {
    caller.data_mut().active_tooltip = None;
}

fn host_insert_text_impl(
    mut caller: wasmi::Caller<ExtensionHostState>,
    text_ptr: u32,
    text_len: u32,
) {
    let text = read_string_from_caller(&caller, text_ptr, text_len);
    caller
        .data_mut()
        .pending_edits
        .push(PendingEdit::Insert(text));
}

fn host_replace_range_impl(
    mut caller: wasmi::Caller<ExtensionHostState>,
    start: u32,
    end: u32,
    text_ptr: u32,
    text_len: u32,
) {
    let text = read_string_from_caller(&caller, text_ptr, text_len);
    caller.data_mut().pending_edits.push(PendingEdit::ReplaceRange {
        start: start as usize,
        end: end as usize,
        text,
    });
}

fn host_create_webview_impl(
    mut caller: wasmi::Caller<ExtensionHostState>,
    url_ptr: u32,
    url_len: u32,
    allow_scripts: u32,
    allow_devtools: u32,
) -> u32 {
    let url = read_string_from_caller(&caller, url_ptr, url_len);
    let data = caller.data_mut();
    let id = data.next_webview_id;
    data.next_webview_id += 1;
    data.pending_webview_requests.push(WebViewRequest {
        webview_id: id.to_string(),
        url,
        method: "GET".into(),
        headers: Vec::new(),
    });
    id
}

fn host_navigate_webview_impl(
    _caller: wasmi::Caller<ExtensionHostState>,
    _webview_id: u32,
    _url_ptr: u32,
    _url_len: u32,
) {
    // Navigation handled by the host UI layer
}

fn host_respond_webview_request_impl(
    mut caller: wasmi::Caller<ExtensionHostState>,
    webview_id: u32,
    mime_ptr: u32,
    mime_len: u32,
    body_ptr: u32,
    body_len: u32,
) {
    let mime_type = read_string_from_caller(&caller, mime_ptr, mime_len);
    let body = {
        let memory = match caller.get_export("memory") {
            Some(wasmi::Extern::Memory(mem)) => mem,
            _ => return,
        };
        let data = memory.data(&caller);
        let start = body_ptr as usize;
        let end = start + body_len as usize;
        if end <= data.len() {
            data[start..end].to_vec()
        } else {
            return;
        }
    };
    caller.data_mut().protocol_responses.insert(
        webview_id,
        ProtocolResponse { mime_type, body },
    );
}
