///|
/// Text widget for displaying text content.
pub struct Text {
  base : Widget
  content : String
  font_size : Double
  font_weight : Double
  color : String
  strikethrough : Bool
}

///|
/// Create a new Text widget with the given content.
pub fn Text::new(content : String) -> Text {
  {
    base: Widget::new("text", "text-gen-1"),
    content: content,
    font_size: 14.0,
    font_weight: 400.0,
    color: "#000000",
    strikethrough: false,
  }
}

///|
/// Create a heading with the given level (1-6).
pub fn Text::heading(content : String, level : Int) -> Text {
  let size = match level {
    1 => 32.0
    2 => 24.0
    3 => 20.0
    4 => 18.0
    5 => 16.0
    _ => 14.0
  }
  {
    base: Widget::new("heading", "heading-gen-1"),
    content: content,
    font_size: size,
    font_weight: 700.0,
    color: "#000000",
    strikethrough: false,
  }
}

///|
/// Set the font size.
pub fn Text::with_font_size(self : Text, size : Double) -> Text {
  { ..self, font_size: size }
}

///|
/// Set the font weight.
pub fn Text::with_font_weight(self : Text, weight : Double) -> Text {
  { ..self, font_weight: weight }
}

///|
/// Set the text color.
pub fn Text::with_color(self : Text, color : String) -> Text {
  { ..self, color: color }
}

///|
/// Set strikethrough decoration.
pub fn Text::strikethrough(self : Text, enabled : Bool) -> Text {
  { ..self, strikethrough: enabled }
}

///|
/// Subscribe to double click events.
pub fn Text::on_double_click(self : Text, handler : () -> Unit) -> Text {
  { ..self, base: self.base.on_double_click(handler) }
}

///|
/// Emit a double click event.
pub fn Text::emit_double_click(self : Text) -> Unit {
  self.base.emit_double_click()
}

///|
/// Get the base widget.
pub fn Text::base(self : Text) -> Widget {
  self.base
}

///|
/// Get the text content.
pub fn Text::get_content(self : Text) -> String {
  self.content
}

///|
/// Get the font size.
pub fn Text::get_font_size(self : Text) -> Double {
  self.font_size
}

///|
/// Get the font weight.
pub fn Text::get_font_weight(self : Text) -> Double {
  self.font_weight
}

///|
/// Get the text color.
pub fn Text::get_color(self : Text) -> String {
  self.color
}

///|
/// Get strikethrough state.
pub fn Text::get_strikethrough(self : Text) -> Bool {
  self.strikethrough
}
