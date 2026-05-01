///|
/// Comprehensive tests for Widget.
test "widget creation" {
  let widget = Widget::new("button", "btn-1")
  assert_eq(widget.get_id(), "btn-1")
  assert_eq(widget.get_type(), "button")
  assert_eq(widget.child_count(), 0)
}

test "widget add child" {
  let child = Widget::new("text", "txt-1")
  let parent = Widget::new("column", "col-1").add_child(child)
  assert_eq(parent.child_count(), 1)
}

test "widget set property" {
  let widget = Widget::new("button", "btn-1")
    .set_property("disabled", "true")
    .set_property("color", "red")
  assert_eq(widget.child_count(), 0)
}

test "widget multiple children" {
  let child1 = Widget::new("text", "txt-1")
  let child2 = Widget::new("text", "txt-2")
  let child3 = Widget::new("text", "txt-3")
  let parent = Widget::new("column", "col-1")
    .add_child(child1)
    .add_child(child2)
    .add_child(child3)
  assert_eq(parent.child_count(), 3)
}
