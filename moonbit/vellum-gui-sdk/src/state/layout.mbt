///|
/// EdgeInsets represents padding or margin for all four sides.
pub struct EdgeInsets {
  top : Double
  right : Double
  bottom : Double
  left : Double
}

///|
/// Create a new EdgeInsets with specific values for each side.
pub fn EdgeInsets::new(top : Double, right : Double, bottom : Double, left : Double) -> EdgeInsets {
  { top: top, right: right, bottom: bottom, left: left }
}

///|
/// Create EdgeInsets with the same value for all sides.
pub fn EdgeInsets::all(value : Double) -> EdgeInsets {
  { top: value, right: value, bottom: value, left: value }
}

///|
/// Create EdgeInsets with horizontal and vertical values.
pub fn EdgeInsets::symmetric(vertical : Double, horizontal : Double) -> EdgeInsets {
  { top: vertical, right: horizontal, bottom: vertical, left: horizontal }
}

///|
/// Create EdgeInsets with only specific sides.
pub fn EdgeInsets::only(top : Double, right : Double, bottom : Double, left : Double) -> EdgeInsets {
  { top: top, right: right, bottom: bottom, left: left }
}

///|
/// Create zero EdgeInsets.
pub fn EdgeInsets::zero() -> EdgeInsets {
  { top: 0.0, right: 0.0, bottom: 0.0, left: 0.0 }
}

///|
/// Get the total horizontal inset (left + right).
pub fn EdgeInsets::horizontal(self : EdgeInsets) -> Double {
  self.left + self.right
}

///|
/// Get the total vertical inset (top + bottom).
pub fn EdgeInsets::vertical(self : EdgeInsets) -> Double {
  self.top + self.bottom
}

///|
/// Get the top inset.
pub fn EdgeInsets::get_top(self : EdgeInsets) -> Double {
  self.top
}

///|
/// Get the right inset.
pub fn EdgeInsets::get_right(self : EdgeInsets) -> Double {
  self.right
}

///|
/// Get the bottom inset.
pub fn EdgeInsets::get_bottom(self : EdgeInsets) -> Double {
  self.bottom
}

///|
/// Get the left inset.
pub fn EdgeInsets::get_left(self : EdgeInsets) -> Double {
  self.left
}

///|
/// Alignment represents horizontal and vertical alignment.
pub enum Alignment {
  TopLeft
  TopCenter
  TopRight
  CenterLeft
  Center
  CenterRight
  BottomLeft
  BottomCenter
  BottomRight
}

///|
/// Constraints represents size constraints for a widget.
pub struct Constraints {
  min_width : Double
  max_width : Double
  min_height : Double
  max_height : Double
}

///|
/// Create new Constraints.
pub fn Constraints::new(min_width : Double, max_width : Double, min_height : Double, max_height : Double) -> Constraints {
  { min_width: min_width, max_width: max_width, min_height: min_height, max_height: max_height }
}

///|
/// Create tight constraints (fixed size).
pub fn Constraints::tight(width : Double, height : Double) -> Constraints {
  { min_width: width, max_width: width, min_height: height, max_height: height }
}

///|
/// Create loose constraints (no minimum).
pub fn Constraints::loose(max_width : Double, max_height : Double) -> Constraints {
  { min_width: 0.0, max_width: max_width, min_height: 0.0, max_height: max_height }
}

///|
/// Create unbounded constraints.
pub fn Constraints::unbounded() -> Constraints {
  { min_width: 0.0, max_width: 1000000000.0, min_height: 0.0, max_height: 1000000000.0 }
}

///|
/// Get the min width.
pub fn Constraints::get_min_width(self : Constraints) -> Double {
  self.min_width
}

///|
/// Get the max width.
pub fn Constraints::get_max_width(self : Constraints) -> Double {
  self.max_width
}

///|
/// Get the min height.
pub fn Constraints::get_min_height(self : Constraints) -> Double {
  self.min_height
}

///|
/// Get the max height.
pub fn Constraints::get_max_height(self : Constraints) -> Double {
  self.max_height
}

///|
/// Layout represents layout properties for a widget.
pub struct Layout {
  padding : EdgeInsets
  margin : EdgeInsets
  width : Double
  height : Double
  flex_grow : Double
  flex_shrink : Double
  alignment : Alignment
}

///|
/// Create a new Layout with default values.
pub fn Layout::new() -> Layout {
  {
    padding: EdgeInsets::zero(),
    margin: EdgeInsets::zero(),
    width: 0.0,
    height: 0.0,
    flex_grow: 0.0,
    flex_shrink: 1.0,
    alignment: Center,
  }
}

///|
/// Set the padding.
pub fn Layout::with_padding(self : Layout, padding : EdgeInsets) -> Layout {
  { ..self, padding: padding }
}

///|
/// Set the margin.
pub fn Layout::with_margin(self : Layout, margin : EdgeInsets) -> Layout {
  { ..self, margin: margin }
}

///|
/// Set the width.
pub fn Layout::with_width(self : Layout, width : Double) -> Layout {
  { ..self, width: width }
}

///|
/// Set the height.
pub fn Layout::with_height(self : Layout, height : Double) -> Layout {
  { ..self, height: height }
}

///|
/// Set the flex grow.
pub fn Layout::with_flex_grow(self : Layout, grow : Double) -> Layout {
  { ..self, flex_grow: grow }
}

///|
/// Set the flex shrink.
pub fn Layout::with_flex_shrink(self : Layout, shrink : Double) -> Layout {
  { ..self, flex_shrink: shrink }
}

///|
/// Set the alignment.
pub fn Layout::with_alignment(self : Layout, alignment : Alignment) -> Layout {
  { ..self, alignment: alignment }
}

///|
/// Get the padding.
pub fn Layout::get_padding(self : Layout) -> EdgeInsets {
  self.padding
}

///|
/// Get the margin.
pub fn Layout::get_margin(self : Layout) -> EdgeInsets {
  self.margin
}

///|
/// Get the width.
pub fn Layout::get_width(self : Layout) -> Double {
  self.width
}

///|
/// Get the height.
pub fn Layout::get_height(self : Layout) -> Double {
  self.height
}

///|
/// Get the flex grow.
pub fn Layout::get_flex_grow(self : Layout) -> Double {
  self.flex_grow
}

///|
/// Get the flex shrink.
pub fn Layout::get_flex_shrink(self : Layout) -> Double {
  self.flex_shrink
}

///|
/// Get the alignment.
pub fn Layout::get_alignment(self : Layout) -> Alignment {
  self.alignment
}
