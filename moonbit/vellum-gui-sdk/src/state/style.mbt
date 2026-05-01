///|
/// Color represents an RGBA color with values from 0.0 to 1.0.
pub struct Color {
  r : Double
  g : Double
  b : Double
  a : Double
}

///|
/// Create a new Color with RGBA values.
pub fn Color::new(r : Double, g : Double, b : Double, a : Double) -> Color {
  { r: r, g: g, b: b, a: a }
}

///|
/// Create a Color from RGB values (alpha defaults to 1.0).
pub fn Color::from_rgb(r : Double, g : Double, b : Double) -> Color {
  { r: r, g: g, b: b, a: 1.0 }
}

///|
/// Create a Color from hex string (e.g., "#FF0000" or "#FF0000FF").
/// Returns Color::black() as fallback for invalid input.
pub fn Color::from_hex(hex : String) -> Color {
  let h_start = if hex.get(0) == Some(35) { 1 } else { 0 } // 35 = '#'
  let r = _hex_char_value(hex.get(0 + h_start)) * 16.0 + _hex_char_value(hex.get(1 + h_start))
  let g = _hex_char_value(hex.get(2 + h_start)) * 16.0 + _hex_char_value(hex.get(3 + h_start))
  let b = _hex_char_value(hex.get(4 + h_start)) * 16.0 + _hex_char_value(hex.get(5 + h_start))
  let a = if hex.length() >= 8 {
    _hex_char_value(hex.get(6 + h_start)) * 16.0 + _hex_char_value(hex.get(7 + h_start))
  } else { 255.0 }
  { r: r / 255.0, g: g / 255.0, b: b / 255.0, a: a / 255.0 }
}

fn _hex_char_value(c : Int?) -> Double {
  match c {
    Some(48) => 0.0  // '0'
    Some(49) => 1.0  // '1'
    Some(50) => 2.0  // '2'
    Some(51) => 3.0  // '3'
    Some(52) => 4.0  // '4'
    Some(53) => 5.0  // '5'
    Some(54) => 6.0  // '6'
    Some(55) => 7.0  // '7'
    Some(56) => 8.0  // '8'
    Some(57) => 9.0  // '9'
    Some(65) | Some(97) => 10.0  // 'A' or 'a'
    Some(66) | Some(98) => 11.0  // 'B' or 'b'
    Some(67) | Some(99) => 12.0  // 'C' or 'c'
    Some(68) | Some(100) => 13.0 // 'D' or 'd'
    Some(69) | Some(101) => 14.0 // 'E' or 'e'
    Some(70) | Some(102) => 15.0 // 'F' or 'f'
    _ => 0.0
  }
}

///|
/// Create a transparent color.
pub fn Color::transparent() -> Color {
  { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }
}

///|
/// Quick helper to create a color with RGBA values.
pub fn color(r : Double, g : Double, b : Double, a : Double) -> Color {
  Color::new(r, g, b, a)
}

///|
/// Create a black color.
pub fn Color::black() -> Color {
  { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }
}

///|
/// Create a white color.
pub fn Color::white() -> Color {
  { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }
}

///|
/// Create a red color.
pub fn Color::red() -> Color {
  { r: 1.0, g: 0.0, b: 0.0, a: 1.0 }
}

///|
/// Create a green color.
pub fn Color::green() -> Color {
  { r: 0.0, g: 1.0, b: 0.0, a: 1.0 }
}

///|
/// Create a blue color.
pub fn Color::blue() -> Color {
  { r: 0.0, g: 0.0, b: 1.0, a: 1.0 }
}

///|
/// Create a gray color.
pub fn Color::gray() -> Color {
  { r: 0.5, g: 0.5, b: 0.5, a: 1.0 }
}

///|
/// Set the alpha value.
pub fn Color::with_alpha(self : Color, alpha : Double) -> Color {
  { ..self, a: alpha }
}

///|
/// Get the red component.
pub fn Color::get_r(self : Color) -> Double {
  self.r
}

///|
/// Get the green component.
pub fn Color::get_g(self : Color) -> Double {
  self.g
}

///|
/// Get the blue component.
pub fn Color::get_b(self : Color) -> Double {
  self.b
}

///|
/// Get the alpha component.
pub fn Color::get_a(self : Color) -> Double {
  self.a
}

///|
/// Border represents a border style.
pub struct Border {
  width : Double
  color : Color
  radius : Double
}

///|
/// Create a new Border.
pub fn Border::new(width : Double, color : Color, radius : Double) -> Border {
  { width: width, color: color, radius: radius }
}

///|
/// Create a border with only width.
pub fn Border::width(width : Double) -> Border {
  { width: width, color: Color::black(), radius: 0.0 }
}

///|
/// Create a border with only radius.
pub fn Border::radius(radius : Double) -> Border {
  { width: 0.0, color: Color::black(), radius: radius }
}

///|
/// Create a border with width and color.
pub fn Border::with_width(self : Border, width : Double) -> Border {
  { ..self, width: width }
}

///|
/// Create a border with color.
pub fn Border::with_color(self : Border, color : Color) -> Border {
  { ..self, color: color }
}

///|
/// Create a border with radius.
pub fn Border::with_radius(self : Border, radius : Double) -> Border {
  { ..self, radius: radius }
}

///|
/// Get the border width.
pub fn Border::get_width(self : Border) -> Double {
  self.width
}

///|
/// Get the border color.
pub fn Border::get_color(self : Border) -> Color {
  self.color
}

///|
/// Get the border radius.
pub fn Border::get_radius(self : Border) -> Double {
  self.radius
}

///|
/// Shadow represents a box shadow style.
pub struct Shadow {
  x : Double
  y : Double
  blur : Double
  color : Color
}

///|
/// Create a new Shadow.
pub fn Shadow::new(x : Double, y : Double, blur : Double, color : Color) -> Shadow {
  { x: x, y: y, blur: blur, color: color }
}

///|
/// Create a shadow with offset only.
pub fn Shadow::offset(x : Double, y : Double) -> Shadow {
  { x: x, y: y, blur: 0.0, color: Color::black().with_alpha(0.5) }
}

///|
/// Create a shadow with blur only.
pub fn Shadow::blur(blur : Double) -> Shadow {
  { x: 0.0, y: 0.0, blur: blur, color: Color::black().with_alpha(0.5) }
}

///|
/// Set the shadow x offset.
pub fn Shadow::with_x(self : Shadow, x : Double) -> Shadow {
  { ..self, x: x }
}

///|
/// Set the shadow y offset.
pub fn Shadow::with_y(self : Shadow, y : Double) -> Shadow {
  { ..self, y: y }
}

///|
/// Set the shadow blur.
pub fn Shadow::with_blur(self : Shadow, blur : Double) -> Shadow {
  { ..self, blur: blur }
}

///|
/// Set the shadow color.
pub fn Shadow::with_color(self : Shadow, color : Color) -> Shadow {
  { ..self, color: color }
}

///|
/// Get the shadow x offset.
pub fn Shadow::get_x(self : Shadow) -> Double {
  self.x
}

///|
/// Get the shadow y offset.
pub fn Shadow::get_y(self : Shadow) -> Double {
  self.y
}

///|
/// Get the shadow blur.
pub fn Shadow::get_blur(self : Shadow) -> Double {
  self.blur
}

///|
/// Get the shadow color.
pub fn Shadow::get_color(self : Shadow) -> Color {
  self.color
}

///|
/// CursorType represents the mouse cursor style.
pub enum CursorType {
  Default
  Pointer
  Text
  Move
  Wait
  Crosshair
  NotAllowed
  Grab
  Grabbing
}

///|
/// Style represents a complete style for a widget.
pub struct Style {
  background : Color
  foreground : Color
  border : Border
  shadow : Shadow
  opacity : Double
  cursor : CursorType
}

///|
/// Create a new Style with default values.
pub fn Style::new() -> Style {
  {
    background: Color::transparent(),
    foreground: Color::black(),
    border: Border::new(0.0, Color::black(), 0.0),
    shadow: Shadow::new(0.0, 0.0, 0.0, Color::transparent()),
    opacity: 1.0,
    cursor: Default,
  }
}

///|
/// Set the background color.
pub fn Style::with_background(self : Style, color : Color) -> Style {
  { ..self, background: color }
}

///|
/// Set the foreground color.
pub fn Style::with_foreground(self : Style, color : Color) -> Style {
  { ..self, foreground: color }
}

///|
/// Set the border.
pub fn Style::with_border(self : Style, border : Border) -> Style {
  { ..self, border: border }
}

///|
/// Set the shadow.
pub fn Style::with_shadow(self : Style, shadow : Shadow) -> Style {
  { ..self, shadow: shadow }
}

///|
/// Set the opacity.
pub fn Style::with_opacity(self : Style, opacity : Double) -> Style {
  { ..self, opacity: opacity }
}

///|
/// Set the cursor.
pub fn Style::with_cursor(self : Style, cursor : CursorType) -> Style {
  { ..self, cursor: cursor }
}

///|
/// Get the background color.
pub fn Style::get_background(self : Style) -> Color {
  self.background
}

///|
/// Get the foreground color.
pub fn Style::get_foreground(self : Style) -> Color {
  self.foreground
}

///|
/// Get the border.
pub fn Style::get_border(self : Style) -> Border {
  self.border
}

///|
/// Get the shadow.
pub fn Style::get_shadow(self : Style) -> Shadow {
  self.shadow
}

///|
/// Get the opacity.
pub fn Style::get_opacity(self : Style) -> Double {
  self.opacity
}

///|
/// Get the cursor.
pub fn Style::get_cursor(self : Style) -> CursorType {
  self.cursor
}
