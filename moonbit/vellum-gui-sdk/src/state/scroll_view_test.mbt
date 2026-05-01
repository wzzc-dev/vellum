///|
/// Tests for ScrollView.
test "scroll view new" {
  let content = Widget::new("column", "content")
  let scroll = ScrollView::new(content)
  assert_eq(scroll.get_scroll_x(), 0.0)
  assert_eq(scroll.get_scroll_y(), 0.0)
  assert_eq(scroll.has_scrollbar(), true)
}

test "scroll view horizontal" {
  let content = Widget::new("column", "content")
  let scroll = ScrollView::horizontal(content)
  match scroll.get_direction() {
    Horizontal => ()
    _ => assert_eq(0, 1)
  }
}

test "scroll view both directions" {
  let content = Widget::new("column", "content")
  let scroll = ScrollView::both(content)
  match scroll.get_direction() {
    Both => ()
    _ => assert_eq(0, 1)
  }
}

test "scroll view with scrollbar" {
  let content = Widget::new("column", "content")
  let scroll = ScrollView::new(content).with_scrollbar(false)
  assert_eq(scroll.has_scrollbar(), false)
}

test "scroll view scroll to" {
  let content = Widget::new("column", "content")
  let scroll = ScrollView::new(content).scroll_to(100.0, 200.0)
  assert_eq(scroll.get_scroll_x(), 100.0)
  assert_eq(scroll.get_scroll_y(), 200.0)
}

test "scroll view scroll by" {
  let content = Widget::new("column", "content")
  let scroll = ScrollView::new(content).scroll_by(10.0, 20.0)
  assert_eq(scroll.get_scroll_x(), 10.0)
  assert_eq(scroll.get_scroll_y(), 20.0)
}

test "scroll view scroll by twice" {
  let content = Widget::new("column", "content")
  let scroll = ScrollView::new(content)
    .scroll_by(10.0, 20.0)
    .scroll_by(5.0, 10.0)
  assert_eq(scroll.get_scroll_x(), 15.0)
  assert_eq(scroll.get_scroll_y(), 30.0)
}

test "scroll view get scroll position" {
  let content = Widget::new("column", "content")
  let scroll = ScrollView::new(content).scroll_to(50.0, 100.0)
  let (x, y) = scroll.get_scroll_position()
  assert_eq(x, 50.0)
  assert_eq(y, 100.0)
}

test "scroll view scroll to top" {
  let content = Widget::new("column", "content")
  let scroll = ScrollView::new(content).scroll_to(0.0, 100.0).scroll_to_top()
  assert_eq(scroll.get_scroll_y(), 0.0)
}

test "scroll view scroll to left" {
  let content = Widget::new("column", "content")
  let scroll = ScrollView::new(content).scroll_to(100.0, 0.0).scroll_to_left()
  assert_eq(scroll.get_scroll_x(), 0.0)
}

test "scroll view base widget" {
  let content = Widget::new("column", "content")
  let scroll = ScrollView::new(content)
  let base = scroll.base()
  assert_eq(base.get_type(), "scroll-view")
}

test "scroll view get content" {
  let content = Widget::new("column", "content")
  let scroll = ScrollView::new(content)
  let c = scroll.get_content()
  assert_eq(c.get_id(), "content")
}
