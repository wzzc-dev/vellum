// vellum:gui/types 类型绑定

pub struct Color {
  r: f32,
  g: f32,
  b: f32,
  a: f32,
} derive(Debug)

pub func color_new(r: f32, g: f32, b: f32, a: f32) -> Color {
  Color:: { r, g, b, a }
}

pub struct Point {
  x: f32,
  y: f32,
} derive(Debug)

pub func point_new(x: f32, y: f32) -> Point {
  Point:: { x, y }
}

pub struct Size {
  width: f32,
  height: f32,
} derive(Debug)

pub func size_new(width: f32, height: f32) -> Size {
  Size:: { width, height }
}

pub struct Rect {
  x: f32,
  y: f32,
  width: f32,
  height: f32,
} derive(Debug)

pub struct EdgeInsets {
  top: f32,
  right: f32,
  bottom: f32,
  left: f32,
} derive(Debug)

pub func edge_insets_new(top: f32, right: f32, bottom: f32, left: f32) -> EdgeInsets {
  EdgeInsets:: { top, right, bottom, left }
}

pub type Alignment = Int
pub type CrossAlignment = Int
pub type CursorShape = Int
pub type Overflow = Int
pub type Visibility = Int
pub type Wrap = Int

pub enum BorderStyle {
  None,
  Solid,
  Dashed,
  Dotted,
  Double,
}

pub struct Border {
  width: f32,
  style: Int,
  color: Color,
}

pub struct BoxShadow {
  offset_x: f32,
  offset_y: f32,
  blur_radius: f32,
  spread_radius: f32,
  color: Color,
}

pub struct Transform {
  translate_x: f32,
  translate_y: f32,
  scale_x: f32,
  scale_y: f32,
  rotation: f32,
}

pub type PointerEvents = Int

pub struct Widget_ {
  id: String,
  widget_type: String,
}
