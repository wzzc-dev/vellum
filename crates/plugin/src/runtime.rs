use anyhow::{Context, Result};
use wasmi::{Engine, Linker, Module, Store};

use crate::abi::HostState;

pub struct PluginRuntime {
    engine: Engine,
    linker: Linker<HostState>,
}

impl PluginRuntime {
    pub fn new() -> Result<Self> {
        let engine = Engine::default();
        let mut linker = <Linker<HostState>>::new(&engine);

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
            .func_wrap("env", "host_register_sidebar_panel", host_register_sidebar_panel_impl)
            .context("failed to define host_register_sidebar_panel")?;
        linker
            .func_wrap("env", "host_subscribe_event", host_subscribe_event_impl)
            .context("failed to define host_subscribe_event")?;
        linker
            .func_wrap("env", "host_set_status_message", host_set_status_message_impl)
            .context("failed to define host_set_status_message")?;
        linker
            .func_wrap("env", "host_get_document_text", host_get_document_text_impl)
            .context("failed to define host_get_document_text")?;
        linker
            .func_wrap("env", "host_get_document_path", host_get_document_path_impl)
            .context("failed to define host_get_document_path")?;
        linker
            .func_wrap("env", "host_set_panel_ui", host_set_panel_ui_impl)
            .context("failed to define host_set_panel_ui")?;
        linker
            .func_wrap("env", "host_set_decorations", host_set_decorations_impl)
            .context("failed to define host_set_decorations")?;
        linker
            .func_wrap("env", "host_clear_decorations", host_clear_decorations_impl)
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
            .func_wrap("env", "host_navigate_webview", host_navigate_webview_impl)
            .context("failed to define host_navigate_webview")?;
        linker
            .func_wrap("env", "host_respond_webview_request", host_respond_webview_request_impl)
            .context("failed to define host_respond_webview_request")?;

        Ok(Self { engine, linker })
    }

    pub fn load_module(&self, wasm_bytes: &[u8]) -> Result<Module> {
        Module::new(&self.engine, wasm_bytes).context("failed to compile WASM module")
    }

    pub fn create_store(&self, state: HostState) -> Store<HostState> {
        Store::new(&self.engine, state)
    }

    pub fn linker(&self) -> &Linker<HostState> {
        &self.linker
    }
}

fn read_bytes_from_caller(caller: &wasmi::Caller<HostState>, ptr: u32, len: u32) -> Vec<u8> {
    let memory = match caller.get_export("memory") {
        Some(wasmi::Extern::Memory(mem)) => mem,
        _ => return Vec::new(),
    };
    let data = memory.data(caller);
    let start = ptr as usize;
    let end = start + len as usize;
    if end <= data.len() {
        data[start..end].to_vec()
    } else {
        Vec::new()
    }
}

fn read_string_from_caller(caller: &wasmi::Caller<HostState>, ptr: u32, len: u32) -> String {
    let bytes = read_bytes_from_caller(caller, ptr, len);
    String::from_utf8_lossy(&bytes).into_owned()
}

fn host_alloc_impl(mut caller: wasmi::Caller<HostState>, size: u32) -> u32 {
    let old_offset = caller.data_mut().alloc_offset;
    caller.data_mut().alloc_offset = old_offset + size;
    old_offset
}

fn host_dealloc_impl(_caller: wasmi::Caller<HostState>, _ptr: u32, _size: u32) {}

fn host_register_command_impl(
    mut caller: wasmi::Caller<HostState>,
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
    data.pending_commands.push(crate::command::RegisteredCommand {
        id: cmd_id,
        command_id: id,
        label,
        key_binding,
        plugin_id: data.plugin_id.clone(),
    });
    cmd_id
}

fn host_register_sidebar_panel_impl(
    mut caller: wasmi::Caller<HostState>,
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
    data.pending_panels.push(crate::decoration::RegisteredPanel {
        id: panel_id,
        panel_id: id,
        label,
        icon: icon_name.into(),
        plugin_id: data.plugin_id.clone(),
    });
    panel_id
}

fn host_subscribe_event_impl(mut caller: wasmi::Caller<HostState>, event_type: u32) -> u32 {
    let data = caller.data_mut();
    let sub_id = data.next_subscription_id;
    data.next_subscription_id += 1;
    data.pending_subscriptions.push((event_type, sub_id));
    sub_id
}

fn host_set_status_message_impl(mut caller: wasmi::Caller<HostState>, msg_ptr: u32, msg_len: u32) {
    let msg = read_string_from_caller(&caller, msg_ptr, msg_len);
    caller.data_mut().status_message = Some(msg);
}

fn host_get_document_text_impl(
    mut caller: wasmi::Caller<HostState>,
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
    mut caller: wasmi::Caller<HostState>,
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
    mut caller: wasmi::Caller<HostState>,
    panel_id: u32,
    ui_ptr: u32,
    ui_len: u32,
) {
    let bytes = read_bytes_from_caller(&caller, ui_ptr, ui_len);
    if let Ok(ui_node) = postcard::from_bytes::<crate::ui::UiNode>(&bytes) {
        caller.data_mut().panel_uis.insert(panel_id, ui_node);
    }
}

fn host_set_decorations_impl(
    mut caller: wasmi::Caller<HostState>,
    decos_ptr: u32,
    decos_len: u32,
) {
    let bytes = read_bytes_from_caller(&caller, decos_ptr, decos_len);
    if let Ok(decorations) = postcard::from_bytes::<Vec<crate::decoration::Decoration>>(&bytes) {
        caller.data_mut().decorations = decorations;
    }
}

fn host_clear_decorations_impl(mut caller: wasmi::Caller<HostState>) {
    caller.data_mut().decorations.clear();
}

fn host_show_overlay_impl(
    mut caller: wasmi::Caller<HostState>,
    overlay_ptr: u32,
    overlay_len: u32,
) {
    let bytes = read_bytes_from_caller(&caller, overlay_ptr, overlay_len);
    if let Ok(overlay) = postcard::from_bytes::<crate::decoration::OverlayPanel>(&bytes) {
        caller.data_mut().active_overlay = Some(overlay);
    }
}

fn host_hide_overlay_impl(mut caller: wasmi::Caller<HostState>, _id_ptr: u32, _id_len: u32) {
    caller.data_mut().active_overlay = None;
}

fn host_show_tooltip_impl(
    mut caller: wasmi::Caller<HostState>,
    _position: u32,
    content_ptr: u32,
    content_len: u32,
) {
    let bytes = read_bytes_from_caller(&caller, content_ptr, content_len);
    if let Ok(tooltip) = postcard::from_bytes::<crate::decoration::Tooltip>(&bytes) {
        caller.data_mut().active_tooltip = Some(tooltip);
    }
}

fn host_hide_tooltip_impl(mut caller: wasmi::Caller<HostState>) {
    caller.data_mut().active_tooltip = None;
}

fn host_insert_text_impl(
    mut caller: wasmi::Caller<HostState>,
    text_ptr: u32,
    text_len: u32,
) {
    let text = read_string_from_caller(&caller, text_ptr, text_len);
    caller.data_mut().pending_edits.push(crate::abi::PendingEdit::Insert(text));
}

fn host_replace_range_impl(
    mut caller: wasmi::Caller<HostState>,
    start: u32,
    end: u32,
    text_ptr: u32,
    text_len: u32,
) {
    let text = read_string_from_caller(&caller, text_ptr, text_len);
    caller.data_mut().pending_edits.push(crate::abi::PendingEdit::ReplaceRange {
        start: start as usize,
        end: end as usize,
        text,
    });
}

fn host_create_webview_impl(
    mut caller: wasmi::Caller<HostState>,
    url_ptr: u32,
    url_len: u32,
    _allow_scripts: u32,
    _allow_devtools: u32,
) -> u32 {
    let url = read_string_from_caller(&caller, url_ptr, url_len);
    let data = caller.data_mut();
    let webview_id = data.next_webview_id;
    data.next_webview_id += 1;
    data.pending_webview_requests.push(crate::decoration::WebViewRequest {
        webview_id,
        url,
    });
    webview_id
}

fn host_navigate_webview_impl(
    _caller: wasmi::Caller<HostState>,
    _webview_id: u32,
    _url_ptr: u32,
    _url_len: u32,
) {
}

fn host_respond_webview_request_impl(
    mut caller: wasmi::Caller<HostState>,
    webview_id: u32,
    mime_ptr: u32,
    mime_len: u32,
    body_ptr: u32,
    body_len: u32,
) {
    let mime_type = read_string_from_caller(&caller, mime_ptr, mime_len);
    let body = read_bytes_from_caller(&caller, body_ptr, body_len);
    caller.data_mut().set_protocol_response(webview_id, crate::decoration::ProtocolResponse {
        mime_type,
        body,
    });
}
