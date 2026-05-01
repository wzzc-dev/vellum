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

test "widget on_click" {
  let mut clicked = false
  let widget = Widget::new("button", "btn-1")
    .on_click(fn() { clicked = true })
  
  widget.emit_click()
  assert(clicked)
}

test "widget on_change" {
  let mut value = ""
  let widget = Widget::new("input", "inp-1")
    .on_change(fn(v) { value = v })
  
  widget.emit_change("new value")
  assert_eq(value, "new value")
}

test "widget on_submit" {
  let mut submitted = ""
  let widget = Widget::new("input", "inp-1")
    .on_submit(fn(v) { submitted = v })
  
  widget.emit_submit("submitted text")
  assert_eq(submitted, "submitted text")
}

test "widget on_blur" {
  let mut blurred = false
  let widget = Widget::new("input", "inp-1")
    .on_blur(fn() { blurred = true })
  
  widget.emit_blur()
  assert(blurred)
}

test "widget on_focus" {
  let mut focused = false
  let widget = Widget::new("input", "inp-1")
    .on_focus(fn() { focused = true })
  
  widget.emit_focus()
  assert(focused)
}

test "widget on_double_click" {
  let mut double_clicked = false
  let widget = Widget::new("text", "txt-1")
    .on_double_click(fn() { double_clicked = true })
  
  widget.emit_double_click()
  assert(double_clicked)
}

test "widget multiple events" {
  let mut results = ""
  let widget = Widget::new("input", "inp-1")
    .on_click(fn() { results = results + "c" })
    .on_focus(fn() { results = results + "f" })
    .on_blur(fn() { results = results + "b" })
  
  widget.emit_click()
  widget.emit_focus()
  widget.emit_blur()
  
  assert_eq(results, "cfb")
}

test "widget get_events" {
  let widget = Widget::new("button", "btn-1")
  let events = widget.get_events()
  
  assert_eq(events.handler_count(), 0)
}

test "widget chained events and properties" {
  let mut clicked = false
  let widget = Widget::new("button", "btn-1")
    .set_property("color", "blue")
    .on_click(fn() { clicked = true })
    .set_property("disabled", "false")
  
  widget.emit_click()
  assert(clicked)
}
