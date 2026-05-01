///|
/// Tests for EdgeInsets.
test "edge insets new" {
  let insets = EdgeInsets::new(10.0, 20.0, 30.0, 40.0)
  assert_eq(insets.get_top(), 10.0)
  assert_eq(insets.get_right(), 20.0)
  assert_eq(insets.get_bottom(), 30.0)
  assert_eq(insets.get_left(), 40.0)
}

test "edge insets all" {
  let insets = EdgeInsets::all(16.0)
  assert_eq(insets.get_top(), 16.0)
  assert_eq(insets.get_right(), 16.0)
  assert_eq(insets.get_bottom(), 16.0)
  assert_eq(insets.get_left(), 16.0)
}

test "edge insets symmetric" {
  let insets = EdgeInsets::symmetric(10.0, 20.0)
  assert_eq(insets.get_top(), 10.0)
  assert_eq(insets.get_right(), 20.0)
  assert_eq(insets.get_bottom(), 10.0)
  assert_eq(insets.get_left(), 20.0)
}

test "edge insets zero" {
  let insets = EdgeInsets::zero()
  assert_eq(insets.get_top(), 0.0)
  assert_eq(insets.get_right(), 0.0)
  assert_eq(insets.get_bottom(), 0.0)
  assert_eq(insets.get_left(), 0.0)
}

test "edge insets horizontal" {
  let insets = EdgeInsets::new(10.0, 20.0, 30.0, 40.0)
  assert_eq(insets.horizontal(), 60.0)
}

test "edge insets vertical" {
  let insets = EdgeInsets::new(10.0, 20.0, 30.0, 40.0)
  assert_eq(insets.vertical(), 40.0)
}

///|
/// Tests for Constraints.
test "constraints new" {
  let constraints = Constraints::new(100.0, 200.0, 50.0, 100.0)
  assert_eq(constraints.get_min_width(), 100.0)
  assert_eq(constraints.get_max_width(), 200.0)
  assert_eq(constraints.get_min_height(), 50.0)
  assert_eq(constraints.get_max_height(), 100.0)
}

test "constraints tight" {
  let constraints = Constraints::tight(100.0, 200.0)
  assert_eq(constraints.get_min_width(), 100.0)
  assert_eq(constraints.get_max_width(), 100.0)
  assert_eq(constraints.get_min_height(), 200.0)
  assert_eq(constraints.get_max_height(), 200.0)
}

test "constraints loose" {
  let constraints = Constraints::loose(200.0, 100.0)
  assert_eq(constraints.get_min_width(), 0.0)
  assert_eq(constraints.get_max_width(), 200.0)
}

test "constraints unbounded" {
  let constraints = Constraints::unbounded()
  assert_eq(constraints.get_min_width(), 0.0)
  assert_eq(constraints.get_min_height(), 0.0)
}

///|
/// Tests for Layout.
test "layout new" {
  let layout = Layout::new()
  assert_eq(layout.get_width(), 0.0)
  assert_eq(layout.get_height(), 0.0)
  assert_eq(layout.get_flex_grow(), 0.0)
}

test "layout with padding" {
  let padding = EdgeInsets::all(16.0)
  let layout = Layout::new().with_padding(padding)
  assert_eq(layout.get_padding().get_top(), 16.0)
}

test "layout with margin" {
  let margin = EdgeInsets::symmetric(8.0, 16.0)
  let layout = Layout::new().with_margin(margin)
  assert_eq(layout.get_margin().get_top(), 8.0)
  assert_eq(layout.get_margin().get_right(), 16.0)
}

test "layout with width" {
  let layout = Layout::new().with_width(200.0)
  assert_eq(layout.get_width(), 200.0)
}

test "layout with height" {
  let layout = Layout::new().with_height(100.0)
  assert_eq(layout.get_height(), 100.0)
}

test "layout with flex grow" {
  let layout = Layout::new().with_flex_grow(1.0)
  assert_eq(layout.get_flex_grow(), 1.0)
}

test "layout with flex shrink" {
  let layout = Layout::new().with_flex_shrink(0.0)
  assert_eq(layout.get_flex_shrink(), 0.0)
}

test "layout chaining" {
  let layout = Layout::new()
    .with_padding(EdgeInsets::all(16.0))
    .with_margin(EdgeInsets::all(8.0))
    .with_width(200.0)
    .with_height(100.0)
    .with_flex_grow(1.0)
  assert_eq(layout.get_padding().get_top(), 16.0)
  assert_eq(layout.get_margin().get_top(), 8.0)
  assert_eq(layout.get_width(), 200.0)
  assert_eq(layout.get_height(), 100.0)
  assert_eq(layout.get_flex_grow(), 1.0)
}
