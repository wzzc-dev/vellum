// Widget types for Vellum GUI Framework

// Core Widget type
pub type Widget = {
  id: String,
  widget_type: String,
  children: List[Widget],
  layout: WidgetLayout,
  props: WidgetProps,
}

pub type WidgetProps = {
  size: Size,
  position: Point,
  padding: EdgeInsets,
  margin: EdgeInsets,
  background: Color,
  opacity: Float,
  visibility: Visibility,
  z_index: I32,
  clip: Bool,
  border_radius: Float,
  shadow: Option[BoxShadow],
}

pub type WidgetLayout = {
  display: WidgetDisplay,
  flex_direction: FlexDirection,
  flex_wrap: Wrap,
  justify_content: Alignment,
  align_items: CrossAlignment,
  align_content: Alignment,
  gap: Float,
  row_gap: Float,
  column_gap: Float,
  position: WidgetPosition,
  min_width: Option[Float],
  max_width: Option[Float],
  min_height: Option[Float],
  max_height: Option[Float],
  overflow_x: Overflow,
  overflow_y: Overflow,
}

// Widget display types
pub enum WidgetDisplay {
  None,
  Flex,
  Block,
  Inline,
  InlineBlock,
  InlineFlex,
  Grid,
}

pub enum FlexDirection {
  Row,
  RowReverse,
  Column,
  ColumnReverse,
}

pub enum Wrap {
  NoWrap,
  Wrap,
  WrapReverse,
}

pub enum WidgetPosition {
  Static,
  Relative,
  Absolute,
  Fixed,
  Sticky,
}

pub enum Visibility {
  Visible,
  Hidden,
  Collapse,
}

// Alignment types
pub enum Alignment {
  Start,
  Center,
  End,
  Stretch,
  SpaceBetween,
  SpaceAround,
  SpaceEvenly,
}

pub enum CrossAlignment {
  Start,
  Center,
  End,
  Stretch,
}

pub enum TextAlign {
  Left,
  Center,
  Right,
  Justify,
}

pub enum VerticalAlign {
  Top,
  Center,
  Bottom,
}

// Color type
pub type Color = {
  r: Float,
  g: Float,
  b: Float,
  a: Float,
}

pub fn color(r: Float, g: Float, b: Float, a: Float) -> Color {
  { r, g, b, a }
}

pub fn rgb(r: Float, g: Float, b: Float) -> Color {
  { r, g, b, a: 1.0 }
}

pub fn rgba(r: Float, g: Float, b: Float, a: Float) -> Color {
  { r, g, b, a }
}

pub fn color_from_hex(hex: String) -> Option[Color] {
  // Parse hex color string
  let hex = hex.replace("#", "")
  if hex.length() == 6 {
    let r = float.from_base(hex.substring(0, 2), 16) / 255.0
    let g = float.from_base(hex.substring(2, 4), 16) / 255.0
    let b = float.from_base(hex.substring(4, 6), 16) / 255.0
    Some({ r, g, b, a: 1.0 })
  } else {
    None
  }
}

// Predefined colors
pub let transparent: Color = { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }
pub let black: Color = { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }
pub let white: Color = { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }
pub let red: Color = { r: 1.0, g: 0.0, b: 0.0, a: 1.0 }
pub let green: Color = { r: 0.0, g: 1.0, b: 0.0, a: 1.0 }
pub let blue: Color = { r: 0.0, g: 0.0, b: 1.0, a: 1.0 }
pub let yellow: Color = { r: 1.0, g: 1.0, b: 0.0, a: 1.0 }
pub let cyan: Color = { r: 0.0, g: 1.0, b: 1.0, a: 1.0 }
pub let magenta: Color = { r: 1.0, g: 0.0, b: 1.0, a: 1.0 }
pub let gray: Color = { r: 0.5, g: 0.5, b: 0.5, a: 1.0 }
pub let light_gray: Color = { r: 0.8, g: 0.8, b: 0.8, a: 1.0 }
pub let dark_gray: Color = { r: 0.3, g: 0.3, b: 0.3, a: 1.0 }

// Geometric types
pub type Point = { x: Float, y: Float }
pub type Size = { width: Float, height: Float }
pub type Rect = { x: Float, y: Float, width: Float, height: Float }

pub type EdgeInsets = {
  top: Float,
  right: Float,
  bottom: Float,
  left: Float,
}

pub fn edge_insets(all: Float) -> EdgeInsets {
  { top: all, right: all, bottom: all, left: all }
}

pub fn edge_insets2(vertical: Float, horizontal: Float) -> EdgeInsets {
  { top: vertical, right: horizontal, bottom: vertical, left: horizontal }
}

pub fn edge_insets4(top: Float, right: Float, bottom: Float, left: Float) -> EdgeInsets {
  { top, right, bottom, left }
}

pub type FlexParams = { grow: Float, shrink: Float, basis: Float }

pub fn flex_none() -> FlexParams {
  { grow: 0.0, shrink: 1.0, basis: 0.0 }
}

pub fn flex_auto() -> FlexParams {
  { grow: 1.0, shrink: 1.0, basis: 0.0 }
}

// Text types
pub type TextStyle = {
  font_size: Float,
  font_family: String,
  font_weight: FontWeight,
  font_style: FontStyle,
  color: Color,
  decoration: TextDecoration,
  letter_spacing: Float,
  line_height: Float,
  text_align: TextAlign,
}

pub enum FontStyle {
  Normal,
  Italic,
}

pub enum FontWeight {
  Thin,
  ExtraLight,
  Light,
  Normal,
  Medium,
  SemiBold,
  Bold,
  ExtraBold,
  Black,
}

pub enum TextDecoration {
  None,
  Underline,
  Overline,
  LineThrough,
}

// Cursor types
pub enum CursorShape {
  Default,
  Pointer,
  Text,
  Wait,
  Crosshair,
  Progress,
  Help,
  Move,
  NotAllowed,
  NoDrop,
  Grab,
  Grabbing,
}

// Overflow types
pub enum Overflow {
  Visible,
  Hidden,
  Scroll,
  Auto,
}

// Border types
pub type Border = {
  width: Float,
  style: BorderStyle,
  color: Color,
}

pub enum BorderStyle {
  None,
  Solid,
  Dashed,
  Dotted,
  Double,
}

// Shadow types
pub type BoxShadow = {
  offset_x: Float,
  offset_y: Float,
  blur_radius: Float,
  spread_radius: Float,
  color: Color,
}

// Transform types
pub type Transform = {
  translate_x: Float,
  translate_y: Float,
  scale_x: Float,
  scale_y: Float,
  rotation: Float,
}

pub fn transform_identity() -> Transform {
  {
    translate_x: 0.0,
    translate_y: 0.0,
    scale_x: 1.0,
    scale_y: 1.0,
    rotation: 0.0,
  }
}

// Widget creation helpers
pub fn widget_new(widget_type: String) -> Widget {
  {
    id: generate_id(),
    widget_type,
    children: [],
    layout: layout_default(),
    props: props_default(),
  }
}

pub fn generate_id() -> String {
  // Generate unique ID using timestamp and random component
  let timestamp = @time.timestamp()
  let random = @random.int(1000000, 9999999)
  "widget_\{timestamp}_\{random}"
}

fn layout_default() -> WidgetLayout {
  {
    display: Flex,
    flex_direction: Row,
    flex_wrap: NoWrap,
    justify_content: Start,
    align_items: Stretch,
    align_content: Start,
    gap: 0.0,
    row_gap: 0.0,
    column_gap: 0.0,
    position: Static,
    min_width: None,
    max_width: None,
    min_height: None,
    max_height: None,
    overflow_x: Visible,
    overflow_y: Visible,
  }
}

fn props_default() -> WidgetProps {
  {
    size: { width: 0.0, height: 0.0 },
    position: { x: 0.0, y: 0.0 },
    padding: edge_insets(0.0),
    margin: edge_insets(0.0),
    background: transparent,
    opacity: 1.0,
    visibility: Visible,
    z_index: 0,
    clip: false,
    border_radius: 0.0,
    shadow: None,
  }
}

// Container widgets
pub fn column(children: List[Widget]) -> Widget {
  {
    id: generate_id(),
    widget_type: "column",
    children,
    layout: {
      display: Flex,
      flex_direction: Column,
      ..layout_default()
    },
    props: props_default(),
  }
}

pub fn row(children: List[Widget]) -> Widget {
  {
    id: generate_id(),
    widget_type: "row",
    children,
    layout: {
      display: Flex,
      flex_direction: Row,
      ..layout_default()
    },
    props: props_default(),
  }
}

pub fn stack(children: List[Widget]) -> Widget {
  {
    id: generate_id(),
    widget_type: "stack",
    children,
    layout: {
      display: Flex,
      flex_direction: Column,
      ..layout_default()
    },
    props: props_default(),
  }
}

pub fn spacer() -> Widget {
  {
    id: generate_id(),
    widget_type: "spacer",
    children: [],
    layout: {
      display: Flex,
      flex_grow: 1.0,
      flex_shrink: 1.0,
      ..layout_default()
    },
    props: props_default(),
  }
}

// Text widgets
pub fn text(content: String) -> Widget {
  {
    id: generate_id(),
    widget_type: "text",
    children: [],
    layout: layout_default(),
    props: {
      props_default() with
      text_content: content,
    },
  }
}

pub fn heading(content: String, level: U8) -> Widget {
  {
    id: generate_id(),
    widget_type: "heading",
    children: [],
    layout: layout_default(),
    props: {
      props_default() with
      text_content: content,
      heading_level: level,
    },
  }
}

pub fn paragraph(content: String) -> Widget {
  {
    id: generate_id(),
    widget_type: "paragraph",
    children: [],
    layout: layout_default(),
    props: {
      props_default() with
      text_content: content,
    },
  }
}

// Button widget
pub type Button = {
  id: String,
  label: String,
  icon: Option[String],
  variant: ButtonVariant,
  disabled: Bool,
}

pub enum ButtonVariant {
  Primary,
  Secondary,
  Ghost,
  Danger,
}

pub fn button(label: String) -> Widget {
  {
    id: generate_id(),
    widget_type: "button",
    children: [{ id: generate_id(), widget_type: "button_text", children: [], layout: layout_default(), props: { props_default() with text_content: label } }],
    layout: layout_default(),
    props: {
      props_default() with
      button_label: label,
      button_variant: Primary,
    },
  }
}

// Input widgets
pub fn text_input(id: String, placeholder: String) -> Widget {
  {
    id,
    widget_type: "text_input",
    children: [],
    layout: layout_default(),
    props: {
      props_default() with
      placeholder,
    },
  }
}

pub fn checkbox(id: String, label: String, checked: Bool) -> Widget {
  {
    id,
    widget_type: "checkbox",
    children: [],
    layout: layout_default(),
    props: {
      props_default() with
      label,
      checked,
    },
  }
}

pub fn toggle(id: String, label: String, active: Bool) -> Widget {
  {
    id,
    widget_type: "toggle",
    children: [],
    layout: layout_default(),
    props: {
      props_default() with
      label,
      active,
    },
  }
}

pub fn select(id: String, options: List[String], selected: Option[U32]) -> Widget {
  {
    id,
    widget_type: "select",
    children: [],
    layout: layout_default(),
    props: {
      props_default() with
      options,
      selected,
    },
  }
}

// Image widget
pub fn image(url: String) -> Widget {
  {
    id: generate_id(),
    widget_type: "image",
    children: [],
    layout: layout_default(),
    props: {
      props_default() with
      image_url: url,
    },
  }
}

pub fn icon(name: String, size: Float) -> Widget {
  {
    id: generate_id(),
    widget_type: "icon",
    children: [],
    layout: layout_default(),
    props: {
      props_default() with
      icon_name: name,
      icon_size: size,
    },
  }
}

// Scroll widgets
pub fn scroll_view(child: Widget) -> Widget {
  {
    id: generate_id(),
    widget_type: "scroll_view",
    children: [child],
    layout: {
      overflow_x: Auto,
      overflow_y: Auto,
      ..layout_default()
    },
    props: props_default(),
  }
}

pub fn list_view(items: List[Widget]) -> Widget {
  {
    id: generate_id(),
    widget_type: "list_view",
    children: items,
    layout: {
      display: Flex,
      flex_direction: Column,
      ..layout_default()
    },
    props: props_default(),
  }
}
