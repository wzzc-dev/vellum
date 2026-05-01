///|
/// ScrollDirection represents the direction of scrolling.
pub enum ScrollDirection {
  Vertical
  Horizontal
  Both
}

///|
/// ScrollView represents a scrollable container.
pub struct ScrollView {
  base : Widget
  content : Widget
  scroll_x : Double
  scroll_y : Double
  show_scrollbar : Bool
  direction : ScrollDirection
}

///|
/// Create a new ScrollView with content (vertical scrolling).
pub fn ScrollView::new(content : Widget) -> ScrollView {
  {
    base: Widget::new("scroll-view", "scroll-gen-1"),
    content: content,
    scroll_x: 0.0,
    scroll_y: 0.0,
    show_scrollbar: true,
    direction: Vertical,
  }
}

///|
/// Create a ScrollView with horizontal scrolling.
pub fn ScrollView::horizontal(content : Widget) -> ScrollView {
  {
    base: Widget::new("scroll-view", "scroll-gen-1"),
    content: content,
    scroll_x: 0.0,
    scroll_y: 0.0,
    show_scrollbar: true,
    direction: Horizontal,
  }
}

///|
/// Create a ScrollView with both directions scrolling.
pub fn ScrollView::both(content : Widget) -> ScrollView {
  {
    base: Widget::new("scroll-view", "scroll-gen-1"),
    content: content,
    scroll_x: 0.0,
    scroll_y: 0.0,
    show_scrollbar: true,
    direction: Both,
  }
}

///|
/// Set the scroll direction.
pub fn ScrollView::with_direction(self : ScrollView, direction : ScrollDirection) -> ScrollView {
  { ..self, direction: direction }
}

///|
/// Set whether to show the scrollbar.
pub fn ScrollView::with_scrollbar(self : ScrollView, show : Bool) -> ScrollView {
  { ..self, show_scrollbar: show }
}

///|
/// Set the scroll position.
pub fn ScrollView::scroll_to(self : ScrollView, x : Double, y : Double) -> ScrollView {
  { ..self, scroll_x: x, scroll_y: y }
}

///|
/// Scroll by a delta amount.
pub fn ScrollView::scroll_by(self : ScrollView, dx : Double, dy : Double) -> ScrollView {
  { ..self, scroll_x: self.scroll_x + dx, scroll_y: self.scroll_y + dy }
}

///|
/// Get the scroll position as a tuple.
pub fn ScrollView::get_scroll_position(self : ScrollView) -> (Double, Double) {
  (self.scroll_x, self.scroll_y)
}

///|
/// Get the horizontal scroll position.
pub fn ScrollView::get_scroll_x(self : ScrollView) -> Double {
  self.scroll_x
}

///|
/// Get the vertical scroll position.
pub fn ScrollView::get_scroll_y(self : ScrollView) -> Double {
  self.scroll_y
}

///|
/// Check if the scrollbar is visible.
pub fn ScrollView::has_scrollbar(self : ScrollView) -> Bool {
  self.show_scrollbar
}

///|
/// Get the content widget.
pub fn ScrollView::get_content(self : ScrollView) -> Widget {
  self.content
}

///|
/// Get the scroll direction.
pub fn ScrollView::get_direction(self : ScrollView) -> ScrollDirection {
  self.direction
}

///|
/// Get the base widget.
pub fn ScrollView::base(self : ScrollView) -> Widget {
  self.base
}

///|
/// Scroll to top.
pub fn ScrollView::scroll_to_top(self : ScrollView) -> ScrollView {
  { ..self, scroll_y: 0.0 }
}

///|
/// Scroll to bottom.
pub fn ScrollView::scroll_to_bottom(self : ScrollView, max_y : Double) -> ScrollView {
  { ..self, scroll_y: max_y }
}

///|
/// Scroll to left.
pub fn ScrollView::scroll_to_left(self : ScrollView) -> ScrollView {
  { ..self, scroll_x: 0.0 }
}

///|
/// Scroll to right.
pub fn ScrollView::scroll_to_right(self : ScrollView, max_x : Double) -> ScrollView {
  { ..self, scroll_x: max_x }
}
