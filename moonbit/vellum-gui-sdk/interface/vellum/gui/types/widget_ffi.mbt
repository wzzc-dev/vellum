// vellum:gui/widget FFI 绑定
//
// 本文件包含与 Rust GPUI Adapter 通过 WIT Component Model 通信的 widget 相关 FFI 函数

fn widget_create(widget_type: String) -> String = "vellum_gui_widget_create"

fn widget_destroy(id: String) -> Void = "vellum_gui_widget_destroy"

fn widget_mount(id: String, parent_id: String) -> Void = "vellum_gui_widget_mount"

fn widget_unmount(id: String) -> Void = "vellum_gui_widget_unmount"

fn widget_set_size(id: String, width: f32, height: f32) -> Void = "vellum_gui_widget_set_size"

fn widget_set_padding(id: String, top: f32, right: f32, bottom: f32, left: f32) -> Void = "vellum_gui_widget_set_padding"

fn widget_set_margin(id: String, top: f32, right: f32, bottom: f32, left: f32) -> Void = "vellum_gui_widget_set_margin"

fn widget_set_background(id: String, r: f32, g: f32, b: f32, a: f32) -> Void = "vellum_gui_widget_set_background"

fn widget_set_opacity(id: String, opacity: f32) -> Void = "vellum_gui_widget_set_opacity"

fn widget_set_visibility(id: String, visible: Bool) -> Void = "vellum_gui_widget_set_visibility"

fn widget_set_cursor(id: String, cursor: Int) -> Void = "vellum_gui_widget_set_cursor"

fn widget_set_border_radius(id: String, radius: f32) -> Void = "vellum_gui_widget_set_border_radius"

fn widget_set_shadow(
  id: String,
  offset_x: f32,
  offset_y: f32,
  blur_radius: f32,
  spread_radius: f32,
  r: f32,
  g: f32,
  b: f32,
  a: f32,
) -> Void = "vellum_gui_widget_set_shadow"

fn widget_mark_needs_layout(id: String) -> Void = "vellum_gui_widget_mark_needs_layout"

fn widget_mark_needs_paint(id: String) -> Void = "vellum_gui_widget_mark_needs_paint"

fn widget_focus(id: String) -> Void = "vellum_gui_widget_focus"

fn widget_blur(id: String) -> Void = "vellum_gui_widget_blur"

fn widget_has_focus(id: String) -> Bool = "vellum_gui_widget_has_focus"
