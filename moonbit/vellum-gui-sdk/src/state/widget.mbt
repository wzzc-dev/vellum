///|
/// Widget represents the base type for all UI components.
pub struct Widget {
  id : String
  widget_type : String
  children : @list.List[Widget]
  properties : @list.List[(String, String)]
  events : EventEmitter
}

///|
/// Create a new Widget with the given type and ID.
pub fn Widget::new(widget_type : String, id : String) -> Widget {
  {
    id: id,
    widget_type: widget_type,
    children: @list.empty(),
    properties: @list.empty(),
    events: EventEmitter::new(),
  }
}

///|
/// Get the ID of this widget.
pub fn Widget::get_id(self : Widget) -> String {
  self.id
}

///|
/// Get the type of this widget.
pub fn Widget::get_type(self : Widget) -> String {
  self.widget_type
}

///|
/// Add a child widget.
pub fn Widget::add_child(self : Widget, child : Widget) -> Widget {
  { ..self, children: self.children.add(child) }
}

///|
/// Set a property on this widget.
pub fn Widget::set_property(self : Widget, key : String, value : String) -> Widget {
  { ..self, properties: self.properties.add((key, value)) }
}

///|
/// Get the number of children.
pub fn Widget::child_count(self : Widget) -> Int {
  self.children.length()
}

///|
/// Subscribe to click events.
pub fn Widget::on_click(self : Widget, handler : () -> Unit) -> Widget {
  { ..self, events: self.events.on_click(handler) }
}

///|
/// Subscribe to change events.
pub fn Widget::on_change(self : Widget, handler : (String) -> Unit) -> Widget {
  { ..self, events: self.events.on_change(handler) }
}

///|
/// Subscribe to submit events.
pub fn Widget::on_submit(self : Widget, handler : (String) -> Unit) -> Widget {
  { ..self, events: self.events.on_submit(handler) }
}

///|
/// Subscribe to input events.
pub fn Widget::on_input(self : Widget, handler : (String) -> Unit) -> Widget {
  { ..self, events: self.events.on_input(handler) }
}

///|
/// Subscribe to blur events.
pub fn Widget::on_blur(self : Widget, handler : () -> Unit) -> Widget {
  { ..self, events: self.events.on_blur(handler) }
}

///|
/// Subscribe to focus events.
pub fn Widget::on_focus(self : Widget, handler : () -> Unit) -> Widget {
  { ..self, events: self.events.on_focus(handler) }
}

///|
/// Subscribe to double click events.
pub fn Widget::on_double_click(self : Widget, handler : () -> Unit) -> Widget {
  { ..self, events: self.events.on_double_click(handler) }
}

///|
/// Subscribe to scroll events.
pub fn Widget::on_scroll(self : Widget, handler : (Double, Double) -> Unit) -> Widget {
  { ..self, events: self.events.on_scroll(handler) }
}

///|
/// Emit a click event.
pub fn Widget::emit_click(self : Widget) -> Unit {
  self.events.emit_click()
}

///|
/// Emit a change event.
pub fn Widget::emit_change(self : Widget, value : String) -> Unit {
  self.events.emit_change(value)
}

///|
/// Emit a submit event.
pub fn Widget::emit_submit(self : Widget, value : String) -> Unit {
  self.events.emit_submit(value)
}

///|
/// Emit a blur event.
pub fn Widget::emit_blur(self : Widget) -> Unit {
  self.events.emit_blur()
}

///|
/// Emit a focus event.
pub fn Widget::emit_focus(self : Widget) -> Unit {
  self.events.emit_focus()
}

///|
/// Emit a double click event.
pub fn Widget::emit_double_click(self : Widget) -> Unit {
  self.events.emit_double_click()
}

///|
/// Get the event emitter.
pub fn Widget::get_events(self : Widget) -> EventEmitter {
  self.events
}
