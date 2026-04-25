use crate::decoration::{Tooltip, TooltipPosition};
use crate::event::EventData;
use crate::host;
use crate::ui::UiEvent;
use crate::manifest::PluginManifest;

pub trait Plugin: Default + 'static {
    fn manifest() -> PluginManifest;
    fn init(&mut self, ctx: &mut PluginContext);
    fn shutdown(&mut self, _ctx: &mut PluginContext) {}
    fn handle_event(&mut self, _event: EventData, _ctx: &mut PluginContext) {}
    fn execute_command(&mut self, _command_id: u32, _ctx: &mut PluginContext) {}
    fn handle_ui_event(&mut self, _event: UiEvent, _ctx: &mut PluginContext) {}
    fn handle_hover(&mut self, hover_data: &str, _ctx: &mut PluginContext) -> Option<Tooltip> {
        let _ = hover_data;
        None
    }
}

pub struct PluginContext;

impl PluginContext {
    pub fn new() -> Self {
        Self
    }

    pub fn register_command(&mut self, id: &str, label: &str, key_binding: Option<&str>) -> u32 {
        host::register_command(id, label, key_binding)
    }

    pub fn register_sidebar_panel(&mut self, id: &str, label: &str, icon: &str) -> u32 {
        host::register_sidebar_panel(id, label, icon)
    }

    pub fn subscribe(&mut self, event_type: crate::event::EventType) {
        host::subscribe(event_type as u32);
    }

    pub fn set_status_message(&mut self, message: &str) {
        host::set_status_message(message);
    }

    pub fn document_text(&self) -> String {
        host::get_document_text()
    }

    pub fn document_path(&self) -> Option<String> {
        host::get_document_path()
    }

    pub fn insert_text(&mut self, text: &str) {
        host::insert_text(text);
    }

    pub fn replace_range(&mut self, start: usize, end: usize, text: &str) {
        host::replace_range(start, end, text);
    }

    pub fn set_panel_ui(&mut self, panel_id: u32, root: crate::ui::UiNode) {
        host::set_panel_ui(panel_id, &root);
    }

    pub fn set_decorations(&mut self, decorations: Vec<crate::decoration::Decoration>) {
        host::set_decorations(&decorations);
    }

    pub fn clear_decorations(&mut self) {
        host::clear_decorations();
    }

    pub fn show_overlay(&mut self, overlay: crate::decoration::OverlayPanel) {
        host::show_overlay(&overlay);
    }

    pub fn hide_overlay(&mut self, id: &str) {
        host::hide_overlay(id);
    }

    pub fn show_tooltip(&mut self, position: TooltipPosition, content: crate::ui::UiNode) {
        host::show_tooltip(position, &content);
    }

    pub fn hide_tooltip(&mut self) {
        host::hide_tooltip();
    }
}

#[macro_export]
macro_rules! vellum_plugin {
    ($plugin_type:ty) => {
        static mut PLUGIN_INSTANCE: Option<$plugin_type> = None;

        #[unsafe(no_mangle)]
        pub extern "C" fn plugin_init() {
            unsafe {
                let mut instance = <$plugin_type>::default();
                let mut ctx = $crate::plugin::PluginContext::new();
                <$plugin_type>::init(&mut instance, &mut ctx);
                PLUGIN_INSTANCE = Some(instance);
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn plugin_shutdown() {
            unsafe {
                if let Some(mut instance) = PLUGIN_INSTANCE.take() {
                    let mut ctx = $crate::plugin::PluginContext::new();
                    <$plugin_type>::shutdown(&mut instance, &mut ctx);
                }
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn plugin_manifest() -> u64 {
            let manifest = <$plugin_type>::manifest();
            let bytes = postcard::to_allocvec(&manifest).unwrap();
            let ptr = bytes.as_ptr() as u32;
            let len = bytes.len() as u32;
            core::mem::forget(bytes);
            ((ptr as u64) << 32) | (len as u64)
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn plugin_handle_event(event_type: u32, data_ptr: u32, data_len: u32) {
            unsafe {
                if let Some(instance) = &mut PLUGIN_INSTANCE {
                    let event_data = $crate::event::decode_event(event_type, data_ptr, data_len);
                    let mut ctx = $crate::plugin::PluginContext::new();
                    <$plugin_type>::handle_event(instance, event_data, &mut ctx);
                }
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn plugin_execute_command(command_id: u32) {
            unsafe {
                if let Some(instance) = &mut PLUGIN_INSTANCE {
                    let mut ctx = $crate::plugin::PluginContext::new();
                    <$plugin_type>::execute_command(instance, command_id, &mut ctx);
                }
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn plugin_handle_ui_event(data_ptr: u32, data_len: u32) {
            unsafe {
                if let Some(instance) = &mut PLUGIN_INSTANCE {
                    let event = $crate::ui::decode_ui_event(data_ptr, data_len);
                    let mut ctx = $crate::plugin::PluginContext::new();
                    <$plugin_type>::handle_ui_event(instance, event, &mut ctx);
                }
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn plugin_handle_hover(hover_data_ptr: u32, hover_data_len: u32) -> u64 {
            unsafe {
                if let Some(instance) = &mut PLUGIN_INSTANCE {
                    let bytes = core::slice::from_raw_parts(hover_data_ptr as *const u8, hover_data_len as usize);
                    let hover_data = core::str::from_utf8(bytes).unwrap_or("");
                    let mut ctx = $crate::plugin::PluginContext::new();
                    let tooltip = <$plugin_type>::handle_hover(instance, hover_data, &mut ctx);
                    match tooltip {
                        Some(t) => {
                            let ser = postcard::to_allocvec(&Some(t)).unwrap();
                            let ptr = ser.as_ptr() as u32;
                            let len = ser.len() as u32;
                            core::mem::forget(ser);
                            ((ptr as u64) << 32) | (len as u64)
                        }
                        None => 0,
                    }
                } else {
                    0
                }
            }
        }
    };
}
