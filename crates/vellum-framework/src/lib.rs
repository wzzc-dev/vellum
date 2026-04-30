//! Vellum GUI Framework - Rust 宿主层
//!
//! 负责：
//! - 原生窗口管理 (GPUI)
//! - 渲染
//! - 事件循环
//! - 系统交互
//! - WASM 组件模型通信

use anyhow::{Context, Result};
use gpui::*;
use std::path::PathBuf;
use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::p2::WasiCtxBuilder;
use wasmtime_wasi::WasiView;

// 从 WIT 生成的绑定
mod bindings {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "framework-world",
    });
}

use bindings::{
    framework_world::{App, HostError},
    vellum::framework::{
        events::{CallbackType, EventCallback},
        renderer::Host as RendererHost,
        host_utils::Host as UtilsHost,
        types::{
            self, Alignment, Border, BorderStyle, Color, CrossAxisAlignment, Decoration,
            FlexDirection, FlexProps, FontWeight, Inset, MainAxisAlignment,
            Point, Rect, Shadow, Size, StackFit, StackProps, TextProps, TextStyle,
        },
        ui_tree::{self, WidgetId, WidgetNode},
    },
};

// ============================================
// 宿主运行时状态
// ============================================
struct FrameworkRuntimeState {
    wasi_ctx: wasmtime_wasi::WasiCtx,
    window_size: Size,
}

impl WasiView for FrameworkRuntimeState {
    fn ctx(&mut self) -> &mut wasmtime_wasi::WasiCtx {
        &mut self.wasi_ctx
    }
    
    fn table(&mut self) -> &mut wasmtime_wasi::ResourceTable {
        unimplemented!("ResourceTable not used")
    }
}

// 实现 Host Utils 接口
impl UtilsHost for FrameworkRuntimeState {
    fn log_debug(&mut self, message: String) -> Result<(), HostError> {
        eprintln!("[Debug] {message}");
        Ok(())
    }

    fn log_info(&mut self, message: String) -> Result<(), HostError> {
        println!("[Info] {message}");
        Ok(())
    }

    fn log_warn(&mut self, message: String) -> Result<(), HostError> {
        eprintln!("[Warning] {message}");
        Ok(())
    }

    fn log_error(&mut self, message: String) -> Result<(), HostError> {
        eprintln!("[Error] {message}");
        Ok(())
    }

    fn rgba(&mut self, r: u8, g: u8, b: u8, a: u8) -> Result<Color, HostError> {
        Ok(Color { r, g, b, a })
    }

    fn hex(&mut self, hex: String) -> Result<Color, HostError> {
        let hex = hex.trim_start_matches('#');
        let mut r = 0u8;
        let mut g = 0u8;
        let mut b = 0u8;
        let mut a = 255u8;
        
        match hex.len() {
            6 => {
                r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| HostError { message: "Invalid hex color".to_string() })?;
                g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| HostError { message: "Invalid hex color".to_string() })?;
                b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| HostError { message: "Invalid hex color".to_string() })?;
            }
            8 => {
                r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| HostError { message: "Invalid hex color".to_string() })?;
                g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| HostError { message: "Invalid hex color".to_string() })?;
                b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| HostError { message: "Invalid hex color".to_string() })?;
                a = u8::from_str_radix(&hex[6..8], 16).map_err(|_| HostError { message: "Invalid hex color".to_string() })?;
            }
            _ => return Err(HostError { message: "Invalid hex color length".to_string() }),
        }
        
        Ok(Color { r, g, b, a })
    }
}

// 实现 Renderer 接口
impl RendererHost for FrameworkRuntimeState {
    fn submit_tree(&mut self, _root: WidgetNode) -> Result<(), HostError> {
        // 树在 AppState 中管理
        Ok(())
    }

    fn request_frame(&mut self) -> Result<(), HostError> {
        // 这里不直接请求帧，通过 AppState
        Ok(())
    }

    fn window_size(&mut self) -> Result<Size, HostError> {
        Ok(self.window_size)
    }
}

// ============================================
// Widget 渲染器
// ============================================
struct WidgetRenderer {
    root: Option<WidgetNode>,
}

impl WidgetRenderer {
    fn new() -> Self {
        Self { root: None }
    }

    fn set_root(&mut self, root: WidgetNode) {
        self.root = Some(root);
    }

    fn render(&self, cx: &mut ViewContext<AppWindow>) -> impl IntoElement {
        match &self.root {
            Some(root) => self.render_widget(root),
            None => div().into_any(),
        }
    }

    fn render_widget(&self, widget: &WidgetNode) -> AnyElement {
        match widget {
            WidgetNode::Container(_id, props, children) => {
                let mut container = div();
                
                if let Some(color) = &props.color {
                    container = container.bg(rgba_to_gpui(color));
                }
                
                if let Some(w) = props.width {
                    container = container.w(w as px);
                }
                
                if let Some(h) = props.height {
                    container = container.h(h as px);
                }
                
                container.children(
                    children.iter().map(|child| self.render_widget(child))
                ).into_any()
            }
            
            WidgetNode::Text(_id, props) => {
                let mut text = text(&props.content);
                
                if let Some(style) = &props.style {
                    text = text.text_size(style.font_size as px);
                    if let Some(color) = &style.color {
                        text = text.text_color(rgba_to_gpui(color));
                    }
                }
                
                text.into_any()
            }
            
            WidgetNode::Column(_id, props, children) => {
                let layout = div()
                    .flex()
                    .flex_col()
                    .children(
                        children.iter().map(|child| self.render_widget(child))
                    );
                layout.into_any()
            }
            
            WidgetNode::Row(_id, props, children) => {
                let layout = div()
                    .flex()
                    .flex_row()
                    .children(
                        children.iter().map(|child| self.render_widget(child))
                    );
                layout.into_any()
            }
            
            WidgetNode::Button(_id, props, children) => {
                let btn = button("")
                    .children(
                        children.iter().map(|child| self.render_widget(child))
                    );
                btn.into_any()
            }
            
            WidgetNode::SizedBox(_id, props, children) => {
                let mut container = div();
                
                if let Some(w) = props.width {
                    container = container.w(w as px);
                }
                
                if let Some(h) = props.height {
                    container = container.h(h as px);
                }
                
                container.children(
                    children.iter().map(|child| self.render_widget(child))
                ).into_any()
            }
            
            _ => div().into_any(),
        }
    }
}

// ============================================
// 应用主窗口
// ============================================
struct AppWindow {
    renderer: WidgetRenderer,
    engine: Engine,
    linker: Linker<FrameworkRuntimeState>,
    store: Option<Store<FrameworkRuntimeState>>,
    app: Option<App>,
    component_path: PathBuf,
    needs_rebuild: bool,
}

impl AppWindow {
    fn new(component_path: PathBuf) -> Result<Self> {
        let mut config = Config::new();
        config.wasm_component_model(true);
        let engine = Engine::new(&config)?;
        
        let mut linker = Linker::new(&engine);
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
        App::add_to_linker(&mut linker, |state| state)?;
        
        Ok(Self {
            renderer: WidgetRenderer::new(),
            engine,
            linker,
            store: None,
            app: None,
            component_path,
            needs_rebuild: true,
        })
    }

    fn load_app(&mut self, cx: &mut ViewContext<Self>) -> Result<()> {
        let component = Component::from_file(&self.engine, &self.component_path)
            .with_context(|| format!("Failed to load component from {}", self.component_path.display()))?;
        
        let state = FrameworkRuntimeState {
            wasi_ctx: WasiCtxBuilder::new().build(),
            window_size: Size { width: 800.0, height: 600.0 },
        };
        
        let mut store = Store::new(&self.engine, state);
        
        let (app, _instance) = App::instantiate(&mut store, &component, &self.linker)
            .context("Failed to instantiate app")?;
        
        // 调用 init
        app.call_init(&mut store)?;
        
        self.store = Some(store);
        self.app = Some(app);
        
        // 第一次构建 UI
        self.rebuild_ui(cx)?;
        
        Ok(())
    }

    fn rebuild_ui(&mut self, cx: &mut ViewContext<Self>) -> Result<()> {
        if let (Some(app), Some(store)) = (&self.app, &mut self.store) {
            let widget = app.call_build_ui(store)?;
            self.renderer.set_root(widget);
            cx.notify();
        }
        Ok(())
    }

    fn handle_event(&mut self, event: EventCallback, cx: &mut ViewContext<Self>) -> Result<()> {
        if let (Some(app), Some(store)) = (&self.app, &mut self.store) {
            let needs_rebuild = app.call_handle_event(store, &event)?;
            if needs_rebuild {
                self.rebuild_ui(cx)?;
            }
        }
        Ok(())
    }

    fn tick(&mut self, delta_ms: u64, cx: &mut ViewContext<Self>) -> Result<()> {
        if let (Some(app), Some(store)) = (&self.app, &mut self.store) {
            let needs_rebuild = app.call_on_tick(store, delta_ms)?;
            if needs_rebuild {
                self.rebuild_ui(cx)?;
            }
        }
        Ok(())
    }
}

impl Render for AppWindow {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        if self.needs_rebuild && self.app.is_none() {
            let _ = self.load_app(cx);
            self.needs_rebuild = false;
        }

        window(
            cx.view().clone(),
            div()
                .flex()
                .flex_col()
                .w_full()
                .h_full()
                .bg(rgb(0x1a1a1a))
                .child(self.renderer.render(cx))
        )
        .title("Vellum Framework App")
    }
}

// ============================================
// 辅助函数
// ============================================
fn rgba_to_gpui(color: &Color) -> Color {
    let c = gpui::rgba(color.r as f32 / 255.0, color.g as f32 / 255.0, color.b as f32 / 255.0, color.a as f32 / 255.0);
    c
}

// ============================================
// 运行应用
// ============================================
pub fn run_app(component_path: PathBuf) -> Result<()> {
    App::new().run(|cx| {
        let app = AppWindow::new(component_path)?;
        cx.new_view(|_| app)
    })?;
    Ok(())
}
