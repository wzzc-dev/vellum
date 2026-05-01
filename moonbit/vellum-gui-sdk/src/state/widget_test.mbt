///|
/// Comprehensive tests for Widget.
test "widget creation" {
  let widget = Widget::new("button", "btn-1")
  inspect(widget.get_id(), content="btn-1")
  inspect(widget.get_type(), content="button")
  inspect(widget.child_count(), content="0")
}

test "widget add child" {
  let child = Widget::new("text", "txt-1")
  let parent = Widget::new("column", "col-1").add_child(child)
  inspect(parent.child_count(), content="1")
}

test "widget set property" {
  let widget = Widget::new("button", "btn-1")
    .set_property("disabled", "true")
    .set_property("color", "red")
  inspect(widget.child_count(), content="0")
}

test "widget multiple children" {
  let child1 = Widget::new("text", "txt-1")
  let child2 = Widget::new("text", "txt-2")
  let child3 = Widget::new("text", "txt-3")
  let parent = Widget::new("column", "col-1")
    .add_child(child1)
    .add_child(child2)
    .add_child(child3)
  inspect(parent.child_count(), content="3")
}

test "widget on_click" {
  let mut clicked = false
  let widget = Widget::new("button", "btn-1")
    .on_click(fn() { clicked = true })
  
  widget.emit_click()
  inspect(clicked, content="true")
}

test "widget on_change" {
  let mut value = ""
  let widget = Widget::new("input", "inp-1")
    .on_change(fn(v) { value = v })
  
  widget.emit_change("new value")
  inspect(value, content="new value")
}

test "widget on_submit" {
  let mut submitted = ""
  let widget = Widget::new("input", "inp-1")
    .on_submit(fn(v) { submitted = v })
  
  widget.emit_submit("submitted text")
  inspect(submitted, content="submitted text")
}

test "widget on_blur" {
  let mut blurred = false
  let widget = Widget::new("input", "inp-1")
    .on_blur(fn() { blurred = true })
  
  widget.emit_blur()
  inspect(blurred, content="true")
}

test "widget on_focus" {
  let mut focused = false
  let widget = Widget::new("input", "inp-1")
    .on_focus(fn() { focused = true })
  
  widget.emit_focus()
  inspect(focused, content="true")
}

test "widget on_double_click" {
  let mut double_clicked = false
  let widget = Widget::new("text", "txt-1")
    .on_double_click(fn() { double_clicked = true })
  
  widget.emit_double_click()
  inspect(double_clicked, content="true")
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
  
  inspect(results, content="cfb")
}

test "widget get_events" {
  let widget = Widget::new("button", "btn-1")
  let events = widget.get_events()
  
  inspect(events.handler_count(), content="0")
}

test "widget chained events and properties" {
  let mut clicked = false
  let widget = Widget::new("button", "btn-1")
    .set_property("color", "blue")
    .on_click(fn() { clicked = true })
    .set_property("disabled", "false")
  
  widget.emit_click()
  inspect(clicked, content="true")
}
