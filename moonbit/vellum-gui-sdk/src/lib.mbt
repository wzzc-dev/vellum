// Vellum GUI Framework SDK for MoonBit
// Main entry point

pub fn init() -> Void {
  // Initialize the GUI framework
  Void
}

// Re-export core types and functions
pub fn @vellum.gui.widget.create_widget(type: String) -> Widget = create_widget
pub fn @vellum.gui.widget.destroy_widget(id: String) -> Void = destroy_widget
pub fn @vellum.gui.widget.mount_widget(id: String, parent_id: String) -> Void = mount_widget
pub fn @vellum.gui.widget.unmount_widget(id: String) -> Void = unmount_widget

pub fn @vellum.gui.window.create_window(options: WindowOptions) -> Option[U32] = create_window
pub fn @vellum.gui.window.close_window(id: U32) -> Void = close_window
pub fn @vellum.gui.window.set_title(id: U32, title: String) -> Void = set_title
pub fn @vellum.gui.window.get_title(id: U32) -> String = get_title

pub fn @vellum.gui.app.create(options: AppOptions) -> AppInstance = create_app
pub fn @vellum.gui.app.run(app: AppInstance) -> Void = run_app
pub fn @vellum.gui.app.quit(app: AppInstance) -> Void = quit_app

// Widget module
pub mod widget {
  pub type Widget = @gui.types.Widget
  pub type WidgetId = String

  pub fn create_widget(type: String) -> WidgetId = @vellum.gui.widget.create_widget
  pub fn destroy_widget(id: WidgetId) -> Void = @vellum.gui.widget.destroy_widget
  pub fn mount_widget(id: WidgetId, parent_id: WidgetId) -> Void = @vellum.gui.widget.mount_widget
  pub fn unmount_widget(id: WidgetId) -> Void = @vellum.gui.widget.unmount_widget

  // Widget property setters
  pub fn set_layout(id: WidgetId, layout: @gui.types.WidgetLayout) -> Void {
    let layout_json = @gui.json.encode_layout(layout)
    @vellum.gui.widget.set_layout(id, layout_json)
  }

  pub fn set_size(id: WidgetId, width: Float, height: Float) -> Void = @vellum.gui.widget.set_size
  pub fn set_position(id: WidgetId, x: Float, y: Float) -> Void = @vellum.gui.widget.set_position
  pub fn set_padding(id: WidgetId, insets: @gui.types.EdgeInsets) -> Void = @vellum.gui.widget.set_padding
  pub fn set_margin(id: WidgetId, insets: @gui.types.EdgeInsets) -> Void = @vellum.gui.widget.set_margin
  pub fn set_background(id: WidgetId, color: @gui.types.Color) -> Void = @vellum.gui.widget.set_background
  pub fn set_opacity(id: WidgetId, opacity: Float) -> Void = @vellum.gui.widget.set_opacity
  pub fn set_visibility(id: WidgetId, visibility: @gui.types.Visibility) -> Void = @vellum.gui.widget.set_visibility
}

// Container module
pub mod container {
  pub fn column(children: List[@gui.widget.Widget]) -> @gui.widget.Widget = @gui.containers.Column::new
  pub fn row(children: List[@gui.widget.Widget]) -> @gui.widget.Widget = @gui.containers.Row::new
  pub fn stack(children: List[@gui.widget.Widget]) -> @gui.widget.Widget = @gui.containers.Stack::new
  pub fn spacer() -> @gui.widget.Widget = @gui.containers.Spacer::new
  pub fn expanded(child: @gui.widget.Widget) -> @gui.widget.Widget = @gui.containers.Expanded::new
}

// Text module
pub mod text {
  pub fn text(content: String) -> @gui.widget.Widget = @gui.text.Text::new
  pub fn heading(content: String, level: U8) -> @gui.widget.Widget = @gui.text.Heading::new
  pub fn paragraph(content: String) -> @gui.widget.Widget = @gui.text.Paragraph::new
  pub fn rich_text(spans: List[@gui.text.TextSpan]) -> @gui.widget.Widget = @gui.text.RichText::new
}

// Button module
pub mod button {
  pub type Button = @gui.button.Button

  pub fn button(label: String) -> Button = @gui.button.Button::new
  pub fn icon_button(icon: String) -> Button = @gui.button.IconButton::new
  pub fn text_button(label: String) -> Button = @gui.button.TextButton::new
}

// Input module
pub mod input {
  pub fn text_input(id: String, placeholder: String) -> @gui.widget.Widget = @gui.input.TextInput::new
  pub fn checkbox(id: String, label: String, checked: Bool) -> @gui.widget.Widget = @gui.input.Checkbox::new
  pub fn toggle(id: String, label: String, active: Bool) -> @gui.widget.Widget = @gui.input.Toggle::new
  pub fn select(id: String, options: List[String]) -> @gui.widget.Widget = @gui.input.Select::new
  pub fn slider(id: String, value: Float, min: Float, max: Float) -> @gui.widget.Widget = @gui.input.Slider::new
  pub fn progress_bar(value: Float) -> @gui.widget.Widget = @gui.input.ProgressBar::new
}

// Image module
pub mod image {
  pub fn image(url: String) -> @gui.widget.Widget = @gui.image.Image::new
  pub fn icon(name: String) -> @gui.widget.Widget = @gui.image.Icon::new
  pub fn avatar(url: String, size: Float) -> @gui.widget.Widget = @gui.image.Avatar::new
}

// Scroll module
pub mod scroll {
  pub fn scroll_view(child: @gui.widget.Widget) -> @gui.widget.Widget = @gui.scroll.ScrollView::new
  pub fn list_view(items: List[@gui.widget.Widget]) -> @gui.widget.Widget = @gui.scroll.ListView::new
  pub fn grid_view(items: List[@gui.widget.Widget], columns: U32) -> @gui.widget.Widget = @gui.scroll.GridView::new
}

// Navigation module
pub mod navigation {
  pub fn tab_bar(tabs: List[@gui.navigation.Tab]) -> @gui.widget.Widget = @gui.navigation.TabBar::new
  pub fn drawer(items: List[@gui.navigation.DrawerItem]) -> @gui.widget.Widget = @gui.navigation.Drawer::new
  pub fn bottom_nav(items: List[@gui.navigation.BottomNavItem]) -> @gui.widget.Widget = @gui.navigation.BottomNav::new
}

// Dialog module
pub mod dialog {
  pub fn alert(message: String, title: String) -> Void = @gui.dialog.alert
  pub fn confirm(message: String, title: String) -> Bool = @gui.dialog.confirm
  pub fn prompt(message: String, title: String, default_value: String) -> Option[String] = @gui.dialog.prompt
  pub fn show_modal(child: @gui.widget.Widget) -> Void = @gui.dialog.show_modal
  pub fn hide_modal() -> Void = @gui.dialog.hide_modal
}

// Event module
pub mod event {
  pub type Event = @gui.events.Event
  pub type MouseEvent = @gui.events.MouseEvent
  pub type KeyEvent = @gui.events.KeyEvent

  pub fn on_click(handler: () -> Void) -> @gui.events.EventHandler = @gui.events.on_click
  pub fn on_input(handler: (String) -> Void) -> @gui.events.EventHandler = @gui.events.on_input
  pub fn on_key_down(handler: (@gui.events.KeyEvent) -> Void) -> @gui.events.EventHandler = @gui.events.on_key_down
  pub fn on_mouse_down(handler: (@gui.events.MouseEvent) -> Void) -> @gui.events.EventHandler = @gui.events.on_mouse_down
  pub fn on_mouse_up(handler: (@gui.events.MouseEvent) -> Void) -> @gui.events.EventHandler = @gui.events.on_mouse_up
  pub fn on_scroll(handler: (@gui.events.ScrollEvent) -> Void) -> @gui.events.EventHandler = @gui.events.on_scroll
}

// Layout module
pub mod layout {
  pub fn padding(all: Float) -> @gui.layout.LayoutModifier = @gui.layout.padding
  pub fn padding2(horizontal: Float, vertical: Float) -> @gui.layout.LayoutModifier = @gui.layout.padding2
  pub fn padding4(top: Float, right: Float, bottom: Float, left: Float) -> @gui.layout.LayoutModifier = @gui.layout.padding4
  pub fn margin(all: Float) -> @gui.layout.LayoutModifier = @gui.layout.margin
  pub fn margin2(horizontal: Float, vertical: Float) -> @gui.layout.LayoutModifier = @gui.layout.margin2
  pub fn margin4(top: Float, right: Float, bottom: Float, left: Float) -> @gui.layout.LayoutModifier = @gui.layout.margin4
  pub fn width(size: Float) -> @gui.layout.LayoutModifier = @gui.layout.width
  pub fn height(size: Float) -> @gui.layout.LayoutModifier = @gui.layout.height
  pub fn flex(grow: Float, shrink: Float, basis: Float) -> @gui.layout.LayoutModifier = @gui.layout.flex
  pub fn flex_grow(grow: Float) -> @gui.layout.LayoutModifier = @gui.layout.flex_grow
  pub fn flex_shrink(shrink: Float) -> @gui.layout.LayoutModifier = @gui.layout.flex_shrink
  pub fn align_self(alignment: @gui.types.Alignment) -> @gui.layout.LayoutModifier = @gui.layout.align_self
}

// Style module
pub mod style {
  pub fn background(color: @gui.types.Color) -> @gui.style.StyleModifier = @gui.style.background
  pub fn foreground(color: @gui.types.Color) -> @gui.style.foreground
  pub fn border(width: Float, color: @gui.types.Color) -> @gui.style.StyleModifier = @gui.style.border
  pub fn border_radius(radius: Float) -> @gui.style.StyleModifier = @gui.style.border_radius
  pub fn shadow(offset_x: Float, offset_y: Float, blur: Float, color: @gui.types.Color) -> @gui.style.StyleModifier = @gui.style.shadow
  pub fn opacity(value: Float) -> @gui.style.StyleModifier = @gui.style.opacity
  pub fn cursor(shape: @gui.types.CursorShape) -> @gui.style.StyleModifier = @gui.style.cursor
}

// Animation module
pub mod animation {
  pub fn fade_in(duration: U32) -> @gui.animation.Animation = @gui.animation.fade_in
  pub fn fade_out(duration: U32) -> @gui.animation.Animation = @gui.animation.fade_out
  pub fn slide_in(direction: @gui.animation.SlideDirection, duration: U32) -> @gui.animation.Animation = @gui.animation.slide_in
  pub fn scale(factor: Float, duration: U32) -> @gui.animation.Animation = @gui.animation.scale
  pub fn rotate(angle: Float, duration: U32) -> @gui.animation.Animation = @gui.animation.rotate
}

// Types module
pub mod types {
  pub type Color = @gui.types.Color
  pub type Point = @gui.types.Point
  pub type Size = @gui.types.Size
  pub type Rect = @gui.types.Rect
  pub type EdgeInsets = @gui.types.EdgeInsets
  pub type FlexParams = @gui.types.FlexParams
  pub type Alignment = @gui.types.Alignment
  pub type CrossAlignment = @gui.types.CrossAlignment
  pub type TextAlign = @gui.types.TextAlign
  pub type VerticalAlign = @gui.types.VerticalAlign
  pub type FontStyle = @gui.types.FontStyle
  pub type FontWeight = @gui.types.FontWeight
  pub type TextDecoration = @gui.types.TextDecoration
  pub type CursorShape = @gui.types.CursorShape
  pub type Overflow = @gui.types.Overflow
  pub type Visibility = @gui.types.Visibility
  pub type WidgetLayout = @gui.types.WidgetLayout
  pub type TextStyle = @gui.types.TextStyle
  pub type Transform = @gui.types.Transform
  pub type BoxShadow = @gui.types.BoxShadow
  pub type Border = @gui.types.Border

  // Color constructors
  pub fn color(r: Float, g: Float, b: Float, a: Float) -> Color = @gui.types.color
  pub fn rgb(r: Float, g: Float, b: Float) -> Color = @gui.types.rgb
  pub fn rgba(r: Float, g: Float, b: Float, a: Float) -> Color = @gui.types.rgba
  pub fn hex(hex: String) -> Option[Color] = @gui.types.color_from_hex

  // Predefined colors
  pub let transparent: Color = @gui.types.transparent
  pub let black: Color = @gui.types.black
  pub let white: Color = @gui.types.white
  pub let red: Color = @gui.types.red
  pub let green: Color = @gui.types.green
  pub let blue: Color = @gui.types.blue
  pub let yellow: Color = @gui.types.yellow
  pub let cyan: Color = @gui.types.cyan
  pub let magenta: Color = @gui.types.magenta
  pub let gray: Color = @gui.types.gray
  pub let light_gray: Color = @gui.types.light_gray
  pub let dark_gray: Color = @gui.types.dark_gray
}

// JSON encoding utilities
pub mod json {
  pub fn encode_color(color: @gui.types.Color) -> String = @gui.json.encode_color
  pub fn encode_layout(layout: @gui.types.WidgetLayout) -> String = @gui.json.encode_layout
  pub fn encode_event(event: @gui.events.Event) -> String = @gui.json.encode_event
}

// Debug utilities
pub mod debug {
  pub fn print_widget_tree(widget: @gui.widget.Widget) -> Void = @gui.debug.print_widget_tree
  pub fn inspect_widget(id: String) -> @gui.types.WidgetInfo = @gui.debug.inspect_widget
  pub fn measure_performance(f: () -> Void) -> Float = @gui.debug.measure_performance
}
