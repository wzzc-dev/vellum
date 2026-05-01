#![cfg(feature = "wit")]

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::ResourceTable;
use wasmtime_wasi::p2::{IoView, WasiCtx, WasiCtxBuilder, WasiView};
use parking_lot::RwLock;

use crate::bridge::GpuiBridge;
use crate::widget::{Widget, WidgetId, WidgetManager};
use crate::types::{Color, Size, Point, Rect, EdgeInsets};

#[allow(dead_code)]
mod bindings {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "vellum-gui",
    });
}

use bindings::VellumGui;
use bindings::vellum::gui::types::*;
use bindings::vellum::gui::widget::Widget as WitWidget;

pub struct GuiRuntimeState {
    bridge: Arc<RwLock<GpuiBridge>>,
    wasi_ctx: WasiCtx,
    resource_table: ResourceTable,
    widget_resources: HashMap<u32, WidgetId>,
    next_widget_id: u32,
}

impl GuiRuntimeState {
    pub fn new(bridge: Arc<RwLock<GpuiBridge>>) -> Self {
        Self {
            bridge,
            wasi_ctx: WasiCtxBuilder::new().build(),
            resource_table: ResourceTable::new(),
            widget_resources: HashMap::new(),
            next_widget_id: 1,
        }
    }
}

impl IoView for GuiRuntimeState {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.resource_table
    }
}

impl WasiView for GuiRuntimeState {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi_ctx
    }
}

impl bindings::vellum::gui::types::Host for GuiRuntimeState {}

impl bindings::vellum::gui::widget::Host for GuiRuntimeState {
    fn create_widget(&mut self, widget_type: String) -> Result<WitWidget, String> {
        let widget_id = self.bridge.read().create_widget(&widget_type);
        let resource_id = self.next_widget_id;
        self.next_widget_id += 1;
        self.widget_resources.insert(resource_id, widget_id.clone());
        Ok(WitWidget {
            id: widget_id.to_string(),
            widget_type,
        })
    }

    fn destroy_widget(&mut self, id: String) -> Result<(), String> {
        if let Ok(widget_id) = WidgetId::from_string(&id) {
            self.bridge.read().destroy_widget(&widget_id);
            self.widget_resources.retain(|_, wid| wid != &widget_id);
        }
        Ok(())
    }

    fn mount_widget(&mut self, id: String, parent_id: String) -> Result<(), String> {
        let widget_id = WidgetId::from_string(&id)
            .map_err(|e| format!("Invalid widget id: {}", e))?;
        let parent_widget_id = WidgetId::from_string(&parent_id)
            .map_err(|e| format!("Invalid parent widget id: {}", e))?;
        self.bridge.read().mount_widget(&widget_id, &parent_widget_id);
        Ok(())
    }

    fn unmount_widget(&mut self, id: String) -> Result<(), String> {
        if let Ok(widget_id) = WidgetId::from_string(&id) {
            self.bridge.read().unmount_widget(&widget_id);
        }
        Ok(())
    }

    fn set_widget_layout(&mut self, id: String, layout: bindings::vellum::gui::widget::WidgetLayout) -> Result<(), String> {
        let widget_id = WidgetId::from_string(&id)
            .map_err(|e| format!("Invalid widget id: {}", e))?;
        
        let bridge = self.bridge.read();
        let widget_manager = bridge.widget_manager();
        
        widget_manager.set_widget_property(&widget_id, "layout", &format!("{:?}", layout));
        
        Ok(())
    }

    fn get_widget_layout(&mut self, id: String) -> Result<bindings::vellum::gui::widget::WidgetLayout, String> {
        let widget_id = WidgetId::from_string(&id)
            .map_err(|e| format!("Invalid widget id: {}", e))?;
        
        Ok(bindings::vellum::gui::widget::WidgetLayout {
            display: bindings::vellum::gui::widget::WidgetDisplay::Flex,
            flex_direction: bindings::vellum::gui::widget::FlexDirection::Row,
            flex_wrap: bindings::vellum::gui::types::Wrap::NoWrap,
            justify_content: bindings::vellum::gui::types::Alignment::Start,
            align_items: bindings::vellum::gui::types::CrossAlignment::Start,
            align_content: bindings::vellum::gui::types::Alignment::Start,
            gap: 0.0,
            row_gap: 0.0,
            column_gap: 0.0,
            position: bindings::vellum::gui::widget::WidgetPosition::Static,
            min_width: None,
            max_width: None,
            min_height: None,
            max_height: None,
            overflow_x: bindings::vellum::gui::types::Overflow::Visible,
            overflow_y: bindings::vellum::gui::types::Overflow::Visible,
        })
    }

    fn set_widget_size(&mut self, id: String, width: f32, height: f32) -> Result<(), String> {
        let widget_id = WidgetId::from_string(&id)
            .map_err(|e| format!("Invalid widget id: {}", e))?;
        
        let bridge = self.bridge.read();
        let widget_manager = bridge.widget_manager();
        
        widget_manager.set_widget_size(&widget_id, Size { width, height });
        
        Ok(())
    }

    fn get_widget_size(&mut self, id: String) -> Result<bindings::vellum::gui::types::Size, String> {
        let widget_id = WidgetId::from_string(&id)
            .map_err(|e| format!("Invalid widget id: {}", e))?;
        
        let bridge = self.bridge.read();
        let widget_manager = bridge.widget_manager();
        let size = widget_manager.get_widget_size(&widget_id);
        
        Ok(bindings::vellum::gui::types::Size {
            width: size.width,
            height: size.height,
        })
    }

    fn set_widget_position(&mut self, id: String, x: f32, y: f32) -> Result<(), String> {
        let widget_id = WidgetId::from_string(&id)
            .map_err(|e| format!("Invalid widget id: {}", e))?;
        
        let bridge = self.bridge.read();
        let widget_manager = bridge.widget_manager();
        
        widget_manager.set_widget_property(&widget_id, "position", &format!("x={}, y={}", x, y));
        
        Ok(())
    }

    fn get_widget_position(&mut self, id: String) -> Result<bindings::vellum::gui::types::Point, String> {
        Ok(bindings::vellum::gui::types::Point { x: 0.0, y: 0.0 })
    }

    fn set_widget_padding(&mut self, id: String, insets: bindings::vellum::gui::types::EdgeInsets) -> Result<(), String> {
        let widget_id = WidgetId::from_string(&id)
            .map_err(|e| format!("Invalid widget id: {}", e))?;
        
        let bridge = self.bridge.read();
        let widget_manager = bridge.widget_manager();
        
        widget_manager.set_widget_padding(&widget_id, EdgeInsets {
            top: insets.top,
            right: insets.right,
            bottom: insets.bottom,
            left: insets.left,
        });
        
        Ok(())
    }

    fn set_widget_margin(&mut self, id: String, insets: bindings::vellum::gui::types::EdgeInsets) -> Result<(), String> {
        let widget_id = WidgetId::from_string(&id)
            .map_err(|e| format!("Invalid widget id: {}", e))?;
        
        let bridge = self.bridge.read();
        let widget_manager = bridge.widget_manager();
        
        widget_manager.set_widget_property(&widget_id, "margin", &format!("{:?}", insets));
        
        Ok(())
    }

    fn set_widget_border(&mut self, id: String, border: bindings::vellum::gui::types::Border) -> Result<(), String> {
        let widget_id = WidgetId::from_string(&id)
            .map_err(|e| format!("Invalid widget id: {}", e))?;
        
        let bridge = self.bridge.read();
        let widget_manager = bridge.widget_manager();
        
        widget_manager.set_widget_property(&widget_id, "border", &format!("{:?}", border));
        
        Ok(())
    }

    fn set_widget_border_radius(&mut self, id: String, radius: f32) -> Result<(), String> {
        let widget_id = WidgetId::from_string(&id)
            .map_err(|e| format!("Invalid widget id: {}", e))?;
        
        let bridge = self.bridge.read();
        let widget_manager = bridge.widget_manager();
        
        widget_manager.set_widget_property(&widget_id, "border-radius", &radius.to_string());
        
        Ok(())
    }

    fn set_widget_background(&mut self, id: String, color: bindings::vellum::gui::types::Color) -> Result<(), String> {
        let widget_id = WidgetId::from_string(&id)
            .map_err(|e| format!("Invalid widget id: {}", e))?;
        
        let bridge = self.bridge.read();
        let widget_manager = bridge.widget_manager();
        
        widget_manager.set_widget_background(&widget_id, Color {
            r: color.r,
            g: color.g,
            b: color.b,
            a: color.a,
        });
        
        Ok(())
    }

    fn set_widget_opacity(&mut self, id: String, opacity: f32) -> Result<(), String> {
        let widget_id = WidgetId::from_string(&id)
            .map_err(|e| format!("Invalid widget id: {}", e))?;
        
        let bridge = self.bridge.read();
        let widget_manager = bridge.widget_manager();
        
        widget_manager.set_widget_property(&widget_id, "opacity", &opacity.to_string());
        
        Ok(())
    }

    fn set_widget_visibility(&mut self, id: String, visibility: bindings::vellum::gui::types::Visibility) -> Result<(), String> {
        let widget_id = WidgetId::from_string(&id)
            .map_err(|e| format!("Invalid widget id: {}", e))?;
        
        let bridge = self.bridge.read();
        let widget_manager = bridge.widget_manager();
        
        widget_manager.set_widget_property(&widget_id, "visibility", &format!("{:?}", visibility));
        
        Ok(())
    }

    fn set_widget_z_index(&mut self, id: String, z_index: i32) -> Result<(), String> {
        let widget_id = WidgetId::from_string(&id)
            .map_err(|e| format!("Invalid widget id: {}", e))?;
        
        let bridge = self.bridge.read();
        let widget_manager = bridge.widget_manager();
        
        widget_manager.set_widget_property(&widget_id, "z-index", &z_index.to_string());
        
        Ok(())
    }

    fn set_widget_cursor(&mut self, id: String, cursor: bindings::vellum::gui::types::CursorShape) -> Result<(), String> {
        let widget_id = WidgetId::from_string(&id)
            .map_err(|e| format!("Invalid widget id: {}", e))?;
        
        let bridge = self.bridge.read();
        let widget_manager = bridge.widget_manager();
        
        widget_manager.set_widget_property(&widget_id, "cursor", &format!("{:?}", cursor));
        
        Ok(())
    }

    fn set_widget_transform(&mut self, id: String, transform: bindings::vellum::gui::types::Transform) -> Result<(), String> {
        let widget_id = WidgetId::from_string(&id)
            .map_err(|e| format!("Invalid widget id: {}", e))?;
        
        let bridge = self.bridge.read();
        let widget_manager = bridge.widget_manager();
        
        widget_manager.set_widget_property(&widget_id, "transform", &format!("{:?}", transform));
        
        Ok(())
    }

    fn set_widget_shadow(&mut self, id: String, shadow: Option<bindings::vellum::gui::types::BoxShadow>) -> Result<(), String> {
        let widget_id = WidgetId::from_string(&id)
            .map_err(|e| format!("Invalid widget id: {}", e))?;
        
        let bridge = self.bridge.read();
        let widget_manager = bridge.widget_manager();
        
        if let Some(shadow) = shadow {
            widget_manager.set_widget_property(&widget_id, "shadow", &format!("{:?}", shadow));
        } else {
            widget_manager.set_widget_property(&widget_id, "shadow", "none");
        }
        
        Ok(())
    }

    fn set_widget_pointer_events(&mut self, id: String, events: bindings::vellum::gui::types::PointerEvents) -> Result<(), String> {
        let widget_id = WidgetId::from_string(&id)
            .map_err(|e| format!("Invalid widget id: {}", e))?;
        
        let bridge = self.bridge.read();
        let widget_manager = bridge.widget_manager();
        
        widget_manager.set_widget_property(&widget_id, "pointer-events", &format!("{:?}", events));
        
        Ok(())
    }

    fn mark_needs_layout(&mut self, id: String) -> Result<(), String> {
        let widget_id = WidgetId::from_string(&id)
            .map_err(|e| format!("Invalid widget id: {}", e))?;
        
        let bridge = self.bridge.read();
        bridge.mark_needs_layout(&widget_id);
        
        Ok(())
    }

    fn mark_needs_paint(&mut self, id: String) -> Result<(), String> {
        let widget_id = WidgetId::from_string(&id)
            .map_err(|e| format!("Invalid widget id: {}", e))?;
        
        let bridge = self.bridge.read();
        bridge.mark_needs_paint(&widget_id);
        
        Ok(())
    }

    fn get_widget_bounds(&mut self, id: String) -> Result<bindings::vellum::gui::types::Rect, String> {
        Ok(bindings::vellum::gui::types::Rect {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        })
    }

    fn get_widget_global_bounds(&mut self, id: String) -> Result<bindings::vellum::gui::types::Rect, String> {
        Ok(bindings::vellum::gui::types::Rect {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        })
    }

    fn set_widget_clip(&mut self, id: String, clip: bool, bounds: bindings::vellum::gui::types::Rect) -> Result<(), String> {
        let widget_id = WidgetId::from_string(&id)
            .map_err(|e| format!("Invalid widget id: {}", e))?;
        
        let bridge = self.bridge.read();
        let widget_manager = bridge.widget_manager();
        
        widget_manager.set_widget_property(&widget_id, "clip", &format!("{} {:?}", clip, bounds));
        
        Ok(())
    }

    fn focus_widget(&mut self, id: String) -> Result<(), String> {
        let widget_id = WidgetId::from_string(&id)
            .map_err(|e| format!("Invalid widget id: {}", e))?;
        
        let bridge = self.bridge.read();
        bridge.focus_widget(&widget_id);
        
        Ok(())
    }

    fn blur_widget(&mut self, id: String) -> Result<(), String> {
        let widget_id = WidgetId::from_string(&id)
            .map_err(|e| format!("Invalid widget id: {}", e))?;
        
        let bridge = self.bridge.read();
        bridge.blur_widget(&widget_id);
        
        Ok(())
    }

    fn has_focus(&mut self, id: String) -> Result<bool, String> {
        let widget_id = WidgetId::from_string(&id)
            .map_err(|e| format!("Invalid widget id: {}", e))?;
        
        let bridge = self.bridge.read();
        Ok(bridge.has_focus(&widget_id))
    }
}

impl bindings::vellum::gui::window::Host for GuiRuntimeState {
    fn create_window(&mut self, _options: bindings::vellum::gui::window::WindowOptions) -> Result<Option<u32>, String> {
        Ok(None)
    }

    fn close_window(&mut self, _id: u32) -> Result<(), String> {
        Ok(())
    }

    fn set_title(&mut self, _id: u32, _title: String) -> Result<(), String> {
        Ok(())
    }

    fn get_title(&mut self, _id: u32) -> Result<String, String> {
        Ok(String::new())
    }

    fn minimize(&mut self, _id: u32) -> Result<(), String> {
        Ok(())
    }

    fn maximize(&mut self, _id: u32) -> Result<(), String> {
        Ok(())
    }

    fn unmaximize(&mut self, _id: u32) -> Result<(), String> {
        Ok(())
    }

    fn is_maximized(&mut self, _id: u32) -> Result<bool, String> {
        Ok(false)
    }

    fn restore(&mut self, _id: u32) -> Result<(), String> {
        Ok(())
    }

    fn show(&mut self, _id: u32) -> Result<(), String> {
        Ok(())
    }

    fn hide(&mut self, _id: u32) -> Result<(), String> {
        Ok(())
    }

    fn is_visible(&mut self, _id: u32) -> Result<bool, String> {
        Ok(true)
    }

    fn set_size(&mut self, _id: u32, _width: u32, _height: u32) -> Result<(), String> {
        Ok(())
    }

    fn get_size(&mut self, _id: u32) -> Result<bindings::vellum::gui::types::Size, String> {
        Ok(bindings::vellum::gui::types::Size { width: 800.0, height: 600.0 })
    }

    fn set_position(&mut self, _id: u32, _x: i32, _y: i32) -> Result<(), String> {
        Ok(())
    }

    fn get_position(&mut self, _id: u32) -> Result<bindings::vellum::gui::types::Point, String> {
        Ok(bindings::vellum::gui::types::Point { x: 0.0, y: 0.0 })
    }

    fn set_fullscreen(&mut self, _id: u32, _fullscreen: bool) -> Result<(), String> {
        Ok(())
    }

    fn is_fullscreen(&mut self, _id: u32) -> Result<bool, String> {
        Ok(false)
    }

    fn set_always_on_top(&mut self, _id: u32, _always_on_top: bool) -> Result<(), String> {
        Ok(())
    }

    fn set_cursor(&mut self, _id: u32, _shape: bindings::vellum::gui::types::CursorShape) -> Result<(), String> {
        Ok(())
    }

    fn set_cursor_position(&mut self, _id: u32, _position: bindings::vellum::gui::types::Point) -> Result<(), String> {
        Ok(())
    }

    fn request_user_attention(&mut self, _id: u32) -> Result<(), String> {
        Ok(())
    }

    fn start_dragging(&mut self, _id: u32) -> Result<(), String> {
        Ok(())
    }
}

impl bindings::vellum::gui::event::Host for GuiRuntimeState {
    fn subscribe(&mut self, _widget_id: String, _event_types: Vec<String>) -> Result<(), String> {
        Ok(())
    }

    fn unsubscribe(&mut self, _widget_id: String) -> Result<(), String> {
        Ok(())
    }

    fn unsubscribe_all(&mut self, _widget_id: String) -> Result<(), String> {
        Ok(())
    }

    fn propagate_to_parent(&mut self, _widget_id: String, _propagate: bool) -> Result<(), String> {
        Ok(())
    }

    fn stop_propagation(&mut self, _event_id: String) -> Result<(), String> {
        Ok(())
    }
}

impl bindings::vellum::gui::paint::Host for GuiRuntimeState {
    fn create_canvas(&mut self, _width: f32, _height: f32) -> Result<bindings::vellum::gui::paint::Canvas, String> {
        Ok(bindings::vellum::gui::paint::Canvas {
            width: _width,
            height: _height,
        })
    }

    fn destroy_canvas(&mut self, _canvas: bindings::vellum::gui::paint::Canvas) -> Result<(), String> {
        Ok(())
    }

    fn begin_paint(&mut self, _canvas: bindings::vellum::gui::paint::Canvas) -> Result<(), String> {
        Ok(())
    }

    fn end_paint(&mut self, _canvas: bindings::vellum::gui::paint::Canvas) -> Result<(), String> {
        Ok(())
    }

    fn clear(&mut self, _canvas: bindings::vellum::gui::paint::Canvas, _color: bindings::vellum::gui::types::Color) -> Result<(), String> {
        Ok(())
    }

    fn save(&mut self, _canvas: bindings::vellum::gui::paint::Canvas) -> Result<(), String> {
        Ok(())
    }

    fn restore(&mut self, _canvas: bindings::vellum::gui::paint::Canvas) -> Result<(), String> {
        Ok(())
    }

    fn translate(&mut self, _canvas: bindings::vellum::gui::paint::Canvas, _x: f32, _y: f32) -> Result<(), String> {
        Ok(())
    }

    fn scale(&mut self, _canvas: bindings::vellum::gui::paint::Canvas, _x: f32, _y: f32) -> Result<(), String> {
        Ok(())
    }

    fn rotate(&mut self, _canvas: bindings::vellum::gui::paint::Canvas, _angle: f32) -> Result<(), String> {
        Ok(())
    }

    fn skew(&mut self, _canvas: bindings::vellum::gui::paint::Canvas, _x: f32, _y: f32) -> Result<(), String> {
        Ok(())
    }

    fn transform(&mut self, _canvas: bindings::vellum::gui::paint::Canvas, _matrix: Vec<f32>) -> Result<(), String> {
        Ok(())
    }

    fn clip_rect(&mut self, _canvas: bindings::vellum::gui::paint::Canvas, _rect: bindings::vellum::gui::types::Rect) -> Result<(), String> {
        Ok(())
    }

    fn clip_path(&mut self, _canvas: bindings::vellum::gui::paint::Canvas, _path: bindings::vellum::gui::paint::Path) -> Result<(), String> {
        Ok(())
    }

    fn reset_clip(&mut self, _canvas: bindings::vellum::gui::paint::Canvas) -> Result<(), String> {
        Ok(())
    }

    fn draw_rect(&mut self, _canvas: bindings::vellum::gui::paint::Canvas, _rect: bindings::vellum::gui::types::Rect, _paint: bindings::vellum::gui::paint::PaintStyle) -> Result<(), String> {
        Ok(())
    }

    fn draw_rounded_rect(&mut self, _canvas: bindings::vellum::gui::paint::Canvas, _rect: bindings::vellum::gui::types::Rect, _radius: f32, _paint: bindings::vellum::gui::paint::PaintStyle) -> Result<(), String> {
        Ok(())
    }

    fn draw_circle(&mut self, _canvas: bindings::vellum::gui::paint::Canvas, _center: bindings::vellum::gui::types::Point, _radius: f32, _paint: bindings::vellum::gui::paint::PaintStyle) -> Result<(), String> {
        Ok(())
    }

    fn draw_ellipse(&mut self, _canvas: bindings::vellum::gui::paint::Canvas, _center: bindings::vellum::gui::types::Point, _radius_x: f32, _radius_y: f32, _paint: bindings::vellum::gui::paint::PaintStyle) -> Result<(), String> {
        Ok(())
    }

    fn draw_line(&mut self, _canvas: bindings::vellum::gui::paint::Canvas, _p1: bindings::vellum::gui::types::Point, _p2: bindings::vellum::gui::types::Point, _stroke: bindings::vellum::gui::paint::StrokeStyle) -> Result<(), String> {
        Ok(())
    }

    fn draw_arc(&mut self, _canvas: bindings::vellum::gui::paint::Canvas, _center: bindings::vellum::gui::types::Point, _radius: f32, _start_angle: f32, _end_angle: f32, _stroke: bindings::vellum::gui::paint::StrokeStyle) -> Result<(), String> {
        Ok(())
    }

    fn draw_path(&mut self, _canvas: bindings::vellum::gui::paint::Canvas, _path: bindings::vellum::gui::paint::Path, _paint: bindings::vellum::gui::paint::PaintStyle) -> Result<(), String> {
        Ok(())
    }

    fn draw_text(&mut self, _canvas: bindings::vellum::gui::paint::Canvas, _text: String, _position: bindings::vellum::gui::types::Point, _style: bindings::vellum::gui::types::TextStyle) -> Result<(), String> {
        Ok(())
    }

    fn measure_text(&mut self, _canvas: bindings::vellum::gui::paint::Canvas, _text: String, _style: bindings::vellum::gui::types::TextStyle) -> Result<bindings::vellum::gui::types::Size, String> {
        Ok(bindings::vellum::gui::types::Size { width: 100.0, height: 20.0 })
    }

    fn measure_text_width(&mut self, _canvas: bindings::vellum::gui::paint::Canvas, _text: String, _style: bindings::vellum::gui::types::TextStyle) -> Result<f32, String> {
        Ok(100.0)
    }

    fn draw_image(&mut self, _canvas: bindings::vellum::gui::paint::Canvas, _image: bindings::vellum::gui::paint::Image, _dest: bindings::vellum::gui::types::Rect, _source: Option<bindings::vellum::gui::types::Rect>) -> Result<(), String> {
        Ok(())
    }

    fn draw_image_resized(&mut self, _canvas: bindings::vellum::gui::paint::Canvas, _image: bindings::vellum::gui::paint::Image, _x: f32, _y: f32, _width: f32, _height: f32) -> Result<(), String> {
        Ok(())
    }

    fn flush(&mut self, _canvas: bindings::vellum::gui::paint::Canvas) -> Result<(), String> {
        Ok(())
    }

    fn to_image(&mut self, _canvas: bindings::vellum::gui::paint::Canvas) -> Result<bindings::vellum::gui::paint::Image, String> {
        Ok(bindings::vellum::gui::paint::Image { width: 100, height: 100 })
    }
}

impl bindings::vellum::gui::Host for GuiRuntimeState {
    fn create_path(&mut self) -> Result<bindings::vellum::gui::paint::Path, String> {
        Ok(bindings::vellum::gui::paint::Path)
    }

    fn destroy_path(&mut self, _path: bindings::vellum::gui::paint::Path) -> Result<(), String> {
        Ok(())
    }

    fn path_move_to(&mut self, _path: bindings::vellum::gui::paint::Path, _x: f32, _y: f32) -> Result<(), String> {
        Ok(())
    }

    fn path_line_to(&mut self, _path: bindings::vellum::gui::paint::Path, _x: f32, _y: f32) -> Result<(), String> {
        Ok(())
    }

    fn path_quadratic_to(&mut self, _path: bindings::vellum::gui::paint::Path, _cp_x: f32, _cp_y: f32, _x: f32, _y: f32) -> Result<(), String> {
        Ok(())
    }

    fn path_cubic_to(&mut self, _path: bindings::vellum::gui::paint::Path, _cp1_x: f32, _cp1_y: f32, _cp2_x: f32, _cp2_y: f32, _x: f32, _y: f32) -> Result<(), String> {
        Ok(())
    }

    fn path_arc_to(&mut self, _path: bindings::vellum::gui::paint::Path, _center: bindings::vellum::gui::types::Point, _radius: f32, _start_angle: f32, _end_angle: f32) -> Result<(), String> {
        Ok(())
    }

    fn path_close(&mut self, _path: bindings::vellum::gui::paint::Path) -> Result<(), String> {
        Ok(())
    }

    fn path_add_rect(&mut self, _path: bindings::vellum::gui::paint::Path, _rect: bindings::vellum::gui::types::Rect) -> Result<(), String> {
        Ok(())
    }

    fn path_add_rounded_rect(&mut self, _path: bindings::vellum::gui::paint::Path, _rect: bindings::vellum::gui::types::Rect, _radius: f32) -> Result<(), String> {
        Ok(())
    }

    fn path_add_circle(&mut self, _path: bindings::vellum::gui::paint::Path, _center: bindings::vellum::gui::types::Point, _radius: f32) -> Result<(), String> {
        Ok(())
    }

    fn path_add_oval(&mut self, _path: bindings::vellum::gui::paint::Path, _rect: bindings::vellum::gui::types::Rect) -> Result<(), String> {
        Ok(())
    }

    fn path_add_path(&mut self, _path: bindings::vellum::gui::paint::Path, _other: bindings::vellum::gui::paint::Path, _offset_x: f32, _offset_y: f32) -> Result<(), String> {
        Ok(())
    }

    fn path_add_text(&mut self, _path: bindings::vellum::gui::paint::Path, _text: String, _position: bindings::vellum::gui::types::Point, _style: bindings::vellum::gui::types::TextStyle) -> Result<(), String> {
        Ok(())
    }

    fn path_is_empty(&mut self, _path: bindings::vellum::gui::paint::Path) -> Result<bool, String> {
        Ok(true)
    }

    fn path_get_bounds(&mut self, _path: bindings::vellum::gui::paint::Path) -> Result<bindings::vellum::gui::types::Rect, String> {
        Ok(bindings::vellum::gui::types::Rect { x: 0.0, y: 0.0, width: 0.0, height: 0.0 })
    }

    fn load_image_from_bytes(&mut self, _bytes: Vec<u8>) -> Result<bindings::vellum::gui::paint::Image, bindings::vellum::gui::paint::PaintError> {
        Ok(bindings::vellum::gui::paint::Image { width: 100, height: 100 })
    }

    fn load_image_from_path(&mut self, _path: String) -> Result<bindings::vellum::gui::paint::Image, bindings::vellum::gui::paint::PaintError> {
        Ok(bindings::vellum::gui::paint::Image { width: 100, height: 100 })
    }

    fn create_image_from_canvas(&mut self, _canvas: bindings::vellum::gui::paint::Canvas) -> Result<bindings::vellum::gui::paint::Image, String> {
        Ok(bindings::vellum::gui::paint::Image { width: 100, height: 100 })
    }

    fn destroy_image(&mut self, _image: bindings::vellum::gui::paint::Image) -> Result<(), String> {
        Ok(())
    }
}

impl bindings::vellum::gui::app::Host for GuiRuntimeState {
    fn create_app(&mut self, _options: bindings::vellum::gui::app::AppOptions) -> Result<bindings::vellum::gui::app::AppInstance, String> {
        Ok(bindings::vellum::gui::app::AppInstance)
    }

    fn destroy_app(&mut self, _app: bindings::vellum::gui::app::AppInstance) -> Result<(), String> {
        Ok(())
    }

    fn run(&mut self, _app: bindings::vellum::gui::app::AppInstance) -> Result<(), String> {
        Ok(())
    }

    fn quit(&mut self, _app: bindings::vellum::gui::app::AppInstance) -> Result<(), String> {
        Ok(())
    }

    fn set_active_window(&mut self, _app: bindings::vellum::gui::app::AppInstance, _window_id: u32) -> Result<(), String> {
        Ok(())
    }

    fn get_active_window(&mut self, _app: bindings::vellum::gui::app::AppInstance) -> Result<Option<u32>, String> {
        Ok(None)
    }

    fn get_all_windows(&mut self, _app: bindings::vellum::gui::app::AppInstance) -> Result<Vec<u32>, String> {
        Ok(Vec::new())
    }

    fn get_window_count(&mut self, _app: bindings::vellum::gui::app::AppInstance) -> Result<u32, String> {
        Ok(0)
    }

    fn get_primary_monitor(&mut self, _app: bindings::vellum::gui::app::AppInstance) -> Result<bindings::vellum::gui::app::MonitorInfo, String> {
        Ok(bindings::vellum::gui::app::MonitorInfo {
            id: 0,
            name: "Primary".to_string(),
            is_primary: true,
            position: bindings::vellum::gui::types::Point { x: 0.0, y: 0.0 },
            size: bindings::vellum::gui::types::Size { width: 1920.0, height: 1080.0 },
            work_area: bindings::vellum::gui::types::Rect { x: 0.0, y: 0.0, width: 1920.0, height: 1080.0 },
            scale_factor: 1.0,
        })
    }

    fn get_all_monitors(&mut self, _app: bindings::vellum::gui::app::AppInstance) -> Result<Vec<bindings::vellum::gui::app::MonitorInfo>, String> {
        Ok(Vec::new())
    }

    fn set_app_theme(&mut self, _app: bindings::vellum::gui::app::AppInstance, _theme: bindings::vellum::gui::app::AppTheme) -> Result<(), String> {
        Ok(())
    }

    fn get_app_theme(&mut self, _app: bindings::vellum::gui::app::AppInstance) -> Result<bindings::vellum::gui::app::AppTheme, String> {
        Ok(bindings::vellum::gui::app::AppTheme::System)
    }

    fn get_system_font(&mut self, _app: bindings::vellum::gui::app::AppInstance) -> Result<String, String> {
        Ok("system-ui".to_string())
    }

    fn get_system_font_size(&mut self, _app: bindings::vellum::gui::app::AppInstance) -> Result<f32, String> {
        Ok(14.0)
    }
}

pub struct GuiHost {
    engine: Engine,
    linker: Linker<GuiRuntimeState>,
}

impl GuiHost {
    pub fn new() -> Result<Self> {
        let mut config = Config::new();
        config.wasm_component_model(true);
        let engine = Engine::new(&config)?;
        let mut linker = Linker::new(&engine);
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
        VellumGui::add_to_linker::<GuiRuntimeState, wasmtime::component::HasSelf<GuiRuntimeState>>(
            &mut linker,
            |state| state,
        )?;

        Ok(Self { engine, linker })
    }

    pub fn instantiate(
        &self,
        bridge: Arc<RwLock<GpuiBridge>>,
        component: &Component,
    ) -> Result<(Store<GuiRuntimeState>, VellumGui)> {
        let mut store = Store::new(&self.engine, GuiRuntimeState::new(bridge));
        let (bindings, _) = VellumGui::instantiate(&mut store, component, &self.linker)?;
        Ok((store, bindings))
    }
}
