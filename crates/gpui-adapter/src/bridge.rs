use crate::error::Result;
use crate::event::{EventDispatcher, GpuiEvent};
use crate::gpui_render::render_widget_tree;
use crate::paint::PaintState;
use crate::types::AppTheme;
use crate::widget::{WidgetManager, WidgetId};
use crate::window::{WindowId, WindowManager, WindowOptions};
use gpui::{Div, Window, Context};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

#[cfg(feature = "mock-gpui")]
use wasmtime::{Config, Engine, Linker, Store};
#[cfg(feature = "mock-gpui")]
use wasmtime_wasi::ResourceTable;
#[cfg(feature = "mock-gpui")]
use wasmtime_wasi::p2::{IoView, WasiCtx, WasiCtxBuilder, WasiView};

pub struct GpuiBridge {
    #[cfg(feature = "mock-gpui")]
    engine: Engine,
    #[cfg(feature = "mock-gpui")]
    linker: Linker<GpuiBridgeState>,
    window_manager: Arc<RwLock<WindowManager>>,
    widget_manager: Arc<RwLock<WidgetManager>>,
    event_dispatcher: Arc<RwLock<EventDispatcher>>,
    paint_state: Arc<RwLock<PaintState>>,
    loaded_apps: HashMap<String, LoadedApp>,
    theme: AppTheme,
}

#[cfg(feature = "mock-gpui")]
struct LoadedApp {
    store: Store<GpuiBridgeState>,
    app_id: String,
}

#[cfg(not(feature = "mock-gpui"))]
struct LoadedApp {
    app_id: String,
    wasm_bytes: Vec<u8>,
}

#[cfg(feature = "mock-gpui")]
pub struct GpuiBridgeState {
    wasi_ctx: WasiCtx,
    resource_table: ResourceTable,
    bridge: Arc<RwLock<Option<Arc<GpuiBridge>>>>,
    theme: AppTheme,
}

#[cfg(feature = "mock-gpui")]
impl GpuiBridgeState {
    fn new() -> Self {
        Self {
            wasi_ctx: WasiCtxBuilder::new().build(),
            resource_table: ResourceTable::new(),
            bridge: Arc::new(RwLock::new(None)),
            theme: AppTheme::System,
        }
    }
}

#[cfg(feature = "mock-gpui")]
impl IoView for GpuiBridgeState {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.resource_table
    }
}

#[cfg(feature = "mock-gpui")]
impl WasiView for GpuiBridgeState {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi_ctx
    }
}

impl GpuiBridge {
    pub fn new() -> Result<Self> {
        #[cfg(feature = "mock-gpui")]
        {
            let mut config = Config::new();
            config.wasm_component_model(true);
            let engine = Engine::new(&config)?;

            let mut linker = Linker::new(&engine);
            wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;

            Ok(Self {
                engine,
                linker,
                window_manager: Arc::new(RwLock::new(WindowManager::new())),
                widget_manager: Arc::new(RwLock::new(WidgetManager::new())),
                event_dispatcher: Arc::new(RwLock::new(EventDispatcher::new())),
                paint_state: Arc::new(RwLock::new(PaintState::new(1024.0, 768.0))),
                loaded_apps: HashMap::new(),
                theme: AppTheme::System,
            })
        }

        #[cfg(not(feature = "mock-gpui"))]
        {
            Ok(Self {
                window_manager: Arc::new(RwLock::new(WindowManager::new())),
                widget_manager: Arc::new(RwLock::new(WidgetManager::new())),
                event_dispatcher: Arc::new(RwLock::new(EventDispatcher::new())),
                paint_state: Arc::new(RwLock::new(PaintState::new(1024.0, 768.0))),
                loaded_apps: HashMap::new(),
                theme: AppTheme::System,
            })
        }
    }

    pub fn create_window(&self, options: WindowOptions) -> WindowId {
        self.window_manager.write().create_window(options)
    }

    pub fn close_window(&self, id: WindowId) -> Result<()> {
        self.window_manager.write().destroy_window(id)
    }

    pub fn set_window_title(&self, id: WindowId, title: String) -> Result<()> {
        self.window_manager.write().set_title(id, title)
    }

    pub fn create_widget(&self, widget_type: &str) -> WidgetId {
        self.widget_manager.write().create_widget(widget_type)
    }

    pub fn destroy_widget(&self, id: &WidgetId) -> Result<()> {
        self.widget_manager.write().destroy_widget(id)
    }

    pub fn mount_widget(&self, id: &WidgetId, parent_id: &WidgetId) -> Result<()> {
        self.widget_manager.write().mount_widget(id, parent_id)
    }

    pub fn unmount_widget(&self, id: &WidgetId) -> Result<()> {
        self.widget_manager.write().unmount_widget(id)
    }

    pub fn set_widget_layout(&self, id: &WidgetId, layout_json: &str) -> Result<()> {
        let layout: crate::types::WidgetLayout = serde_json::from_str(layout_json)?;
        self.widget_manager.write().set_layout(id, layout)
    }

    pub fn set_widget_size(&self, id: &WidgetId, width: f32, height: f32) -> Result<()> {
        self.widget_manager.write().set_size(id, width, height)
    }

    pub fn set_widget_position(&self, id: &WidgetId, x: f32, y: f32) -> Result<()> {
        self.widget_manager.write().set_position(id, x, y)
    }

    pub fn set_widget_background(&self, id: &WidgetId, color_json: &str) -> Result<()> {
        let color: crate::types::Color = serde_json::from_str(color_json)?;
        self.widget_manager.write().set_background(id, color)
    }

    pub fn set_widget_opacity(&self, id: &WidgetId, opacity: f32) -> Result<()> {
        self.widget_manager.write().set_opacity(id, opacity)
    }

    pub fn set_widget_visibility(&self, id: &WidgetId, visibility_json: &str) -> Result<()> {
        let visibility: crate::types::Visibility = serde_json::from_str(visibility_json)?;
        self.widget_manager.write().set_visibility(id, visibility)
    }

    pub fn get_widget_bounds(&self, id: &WidgetId) -> Result<crate::types::Rect> {
        self.widget_manager.read().get_bounds(id)
    }

    pub fn get_widget_global_bounds(&self, id: &WidgetId) -> Result<crate::types::Rect> {
        self.widget_manager.read().get_global_bounds(id)
    }

    pub fn subscribe_event(&self, widget_id: &WidgetId, event_types: Vec<String>) {
        for event_type in event_types {
            let widget_id = widget_id.clone();
            let event_dispatcher = self.event_dispatcher.clone();

            self.event_dispatcher.write().subscribe(
                &widget_id,
                Box::new(move |event: &GpuiEvent| {
                    let _ = event_dispatcher.write().dispatch(event);
                }),
            );
        }
    }

    pub fn unsubscribe_events(&self, widget_id: &WidgetId) {
        self.event_dispatcher.write().unsubscribe(widget_id);
    }

    pub fn dispatch_event(&self, event: GpuiEvent) {
        self.event_dispatcher.read().dispatch(&event);
    }

    pub fn get_window_manager(&self) -> Arc<RwLock<WindowManager>> {
        self.window_manager.clone()
    }

    pub fn get_widget_manager(&self) -> Arc<RwLock<WidgetManager>> {
        self.widget_manager.clone()
    }

    pub fn get_paint_state(&self) -> Arc<RwLock<PaintState>> {
        self.paint_state.clone()
    }

    pub fn set_theme(&mut self, theme: AppTheme) {
        self.theme = theme;
    }

    pub fn get_theme(&self) -> AppTheme {
        self.theme
    }

    /// 渲染 Widget 树为 GPUI 元素
    pub fn render_widget_tree(
        &self,
        root_id: &WidgetId,
        window: &mut Window,
        cx: &mut Context,
    ) -> Div {
        render_widget_tree(&self.widget_manager, root_id, window, cx)
    }

    /// 创建示例计数器 Widget 树（用于测试）
    pub fn create_example_counter(&self) -> WidgetId {
        let column_id = self.create_widget("column");
        
        let text_id = self.create_widget("text");
        self.widget_manager
            .write()
            .set_widget_property(&text_id, "content", "Count: 0".to_string());
        
        let button_id = self.create_widget("button");
        self.widget_manager
            .write()
            .set_widget_property(&button_id, "label", "Increment".to_string());
        
        self.mount_widget(&text_id, &column_id).unwrap();
        self.mount_widget(&button_id, &column_id).unwrap();
        
        column_id
    }

    #[cfg(feature = "mock-gpui")]
    pub fn load_wasm_component(&mut self, app_id: &str, wasm_bytes: &[u8]) -> Result<()> {
        use wasmtime::component::Component;

        let component = Component::from_binary(&self.engine, wasm_bytes)?;

        let state = GpuiBridgeState::new();
        let mut store = Store::new(&self.engine, state);

        let instance = self.linker.instantiate(&mut store, &component)?;

        self.loaded_apps.insert(
            app_id.to_string(),
            LoadedApp {
                store,
                app_id: app_id.to_string(),
            },
        );

        Ok(())
    }

    #[cfg(not(feature = "mock-gpui"))]
    pub fn load_wasm_component(&mut self, app_id: &str, wasm_bytes: &[u8]) -> Result<()> {
        self.loaded_apps.insert(
            app_id.to_string(),
            LoadedApp {
                app_id: app_id.to_string(),
                wasm_bytes: wasm_bytes.to_vec(),
            },
        );
        Ok(())
    }

    pub fn get_loaded_app(&self, app_id: &str) -> Option<&LoadedApp> {
        self.loaded_apps.get(app_id)
    }

    pub fn unload_app(&mut self, app_id: &str) -> Option<LoadedApp> {
        self.loaded_apps.remove(app_id)
    }

    pub fn list_loaded_apps(&self) -> Vec<String> {
        self.loaded_apps.keys().cloned().collect()
    }
}

impl Default for GpuiBridge {
    fn default() -> Self {
        Self::new().expect("failed to create GpuiBridge")
    }
}

pub trait WidgetBuilder {
    fn build_column(&self) -> WidgetId;
    fn build_row(&self) -> WidgetId;
    fn build_stack(&self) -> WidgetId;
    fn build_text(&self, content: &str) -> WidgetId;
    fn build_button(&self, id: &str, label: &str) -> WidgetId;
    fn build_image(&self, url: &str) -> WidgetId;
    fn build_input(&self, id: &str, placeholder: &str) -> WidgetId;
}

impl WidgetBuilder for GpuiBridge {
    fn build_column(&self) -> WidgetId {
        self.create_widget("column")
    }

    fn build_row(&self) -> WidgetId {
        self.create_widget("row")
    }

    fn build_stack(&self) -> WidgetId {
        self.create_widget("stack")
    }

    fn build_text(&self, _content: &str) -> WidgetId {
        self.create_widget("text")
    }

    fn build_button(&self, _id: &str, _label: &str) -> WidgetId {
        self.create_widget("button")
    }

    fn build_image(&self, _url: &str) -> WidgetId {
        self.create_widget("image")
    }

    fn build_input(&self, _id: &str, _placeholder: &str) -> WidgetId {
        self.create_widget("input")
    }
}

pub struct GpuiBridgeBuilder {
    theme: AppTheme,
}

impl GpuiBridgeBuilder {
    pub fn new() -> Self {
        Self {
            theme: AppTheme::System,
        }
    }

    pub fn with_theme(mut self, theme: AppTheme) -> Self {
        self.theme = theme;
        self
    }

    pub fn build(self) -> Result<GpuiBridge> {
        let bridge = GpuiBridge::new()?;
        let mut bridge = bridge;
        bridge.set_theme(self.theme);
        Ok(bridge)
    }
}

impl Default for GpuiBridgeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_creation() {
        let bridge = GpuiBridge::new();
        assert!(bridge.is_ok());
    }

    #[test]
    fn test_window_creation() {
        let bridge = GpuiBridge::new().unwrap();
        let window_id = bridge.create_window(WindowOptions::default());
        assert!(window_id >= 0); // First window ID is 0
    }

    #[test]
    fn test_widget_creation() {
        let bridge = GpuiBridge::new().unwrap();
        let widget_id = bridge.create_widget("column");
        assert!(!widget_id.is_empty());
    }

    #[test]
    fn test_widget_mounting() {
        let bridge = GpuiBridge::new().unwrap();
        let parent_id = bridge.create_widget("column");
        let child_id = bridge.create_widget("text");

        let result = bridge.mount_widget(&child_id, &parent_id);
        assert!(result.is_ok());

        let manager = bridge.widget_manager.read();
        let children = manager.get_children(&parent_id);
        assert!(children.is_some());
        assert_eq!(children.unwrap().len(), 1);
    }

    #[test]
    fn test_widget_hierarchy() {
        let bridge = GpuiBridge::new().unwrap();

        let column_id = bridge.create_widget("column");
        let row_id = bridge.create_widget("row");
        let text_id = bridge.create_widget("text");

        bridge.mount_widget(&row_id, &column_id).unwrap();
        bridge.mount_widget(&text_id, &row_id).unwrap();

        let manager = bridge.widget_manager.read();
        let row_children = manager.get_children(&row_id);
        assert_eq!(row_children.unwrap().len(), 1);

        let column_children = manager.get_children(&column_id);
        assert_eq!(column_children.unwrap().len(), 1);
    }

    #[test]
    fn test_widget_destruction() {
        let bridge = GpuiBridge::new().unwrap();

        let parent_id = bridge.create_widget("column");
        let child_id = bridge.create_widget("text");

        bridge.mount_widget(&child_id, &parent_id).unwrap();
        bridge.destroy_widget(&parent_id).unwrap();

        let manager = bridge.widget_manager.read();
        assert!(manager.get_widget(&parent_id).is_none());
        assert!(manager.get_widget(&child_id).is_none());
    }

    #[test]
    fn test_widget_properties() {
        let bridge = GpuiBridge::new().unwrap();
        let widget_id = bridge.create_widget("button");

        bridge.set_widget_size(&widget_id, 100.0, 50.0).unwrap();
        bridge.set_widget_position(&widget_id, 10.0, 20.0).unwrap();

        let manager = bridge.widget_manager.read();
        let size = manager.get_size(&widget_id).unwrap();
        assert_eq!(size.width, 100.0);
        assert_eq!(size.height, 50.0);

        let position = manager.get_position(&widget_id).unwrap();
        assert_eq!(position.0, 10.0);
        assert_eq!(position.1, 20.0);
    }
}
