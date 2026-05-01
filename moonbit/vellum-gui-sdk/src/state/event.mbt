///|
/// EventEmitter manages event subscriptions and dispatching with actual callbacks.
pub struct EventEmitter {
  mut click_handlers : @list.List[() -> Unit]
  mut change_handlers : @list.List[(String) -> Unit]
  mut submit_handlers : @list.List[(String) -> Unit]
  mut input_handlers : @list.List[(String) -> Unit]
  mut blur_handlers : @list.List[() -> Unit]
  mut focus_handlers : @list.List[() -> Unit]
  mut double_click_handlers : @list.List[() -> Unit]
  mut scroll_handlers : @list.List[(Double, Double) -> Unit]
}

///|
/// Create a new EventEmitter.
pub fn EventEmitter::new() -> EventEmitter {
  {
    click_handlers: @list.empty(),
    change_handlers: @list.empty(),
    submit_handlers: @list.empty(),
    input_handlers: @list.empty(),
    blur_handlers: @list.empty(),
    focus_handlers: @list.empty(),
    double_click_handlers: @list.empty(),
    scroll_handlers: @list.empty(),
  }
}

///|
/// Subscribe to click events.
pub fn EventEmitter::on_click(self : EventEmitter, handler : () -> Unit) -> EventEmitter {
  self.click_handlers = self.click_handlers.add(handler)
  self
}

///|
/// Subscribe to change events.
pub fn EventEmitter::on_change(self : EventEmitter, handler : (String) -> Unit) -> EventEmitter {
  self.change_handlers = self.change_handlers.add(handler)
  self
}

///|
/// Subscribe to submit events.
pub fn EventEmitter::on_submit(self : EventEmitter, handler : (String) -> Unit) -> EventEmitter {
  self.submit_handlers = self.submit_handlers.add(handler)
  self
}

///|
/// Subscribe to input events.
pub fn EventEmitter::on_input(self : EventEmitter, handler : (String) -> Unit) -> EventEmitter {
  self.input_handlers = self.input_handlers.add(handler)
  self
}

///|
/// Subscribe to blur events.
pub fn EventEmitter::on_blur(self : EventEmitter, handler : () -> Unit) -> EventEmitter {
  self.blur_handlers = self.blur_handlers.add(handler)
  self
}

///|
/// Subscribe to focus events.
pub fn EventEmitter::on_focus(self : EventEmitter, handler : () -> Unit) -> EventEmitter {
  self.focus_handlers = self.focus_handlers.add(handler)
  self
}

///|
/// Subscribe to double click events.
pub fn EventEmitter::on_double_click(self : EventEmitter, handler : () -> Unit) -> EventEmitter {
  self.double_click_handlers = self.double_click_handlers.add(handler)
  self
}

///|
/// Subscribe to scroll events.
pub fn EventEmitter::on_scroll(self : EventEmitter, handler : (Double, Double) -> Unit) -> EventEmitter {
  self.scroll_handlers = self.scroll_handlers.add(handler)
  self
}

///|
/// Emit a click event.
pub fn EventEmitter::emit_click(self : EventEmitter) -> Unit {
  self.click_handlers.each(fn(handler) { handler() })
}

///|
/// Emit a change event.
pub fn EventEmitter::emit_change(self : EventEmitter, value : String) -> Unit {
  self.change_handlers.each(fn(handler) { handler(value) })
}

///|
/// Emit a submit event.
pub fn EventEmitter::emit_submit(self : EventEmitter, value : String) -> Unit {
  self.submit_handlers.each(fn(handler) { handler(value) })
}

///|
/// Emit an input event.
pub fn EventEmitter::emit_input(self : EventEmitter, value : String) -> Unit {
  self.input_handlers.each(fn(handler) { handler(value) })
}

///|
/// Emit a blur event.
pub fn EventEmitter::emit_blur(self : EventEmitter) -> Unit {
  self.blur_handlers.each(fn(handler) { handler() })
}

///|
/// Emit a focus event.
pub fn EventEmitter::emit_focus(self : EventEmitter) -> Unit {
  self.focus_handlers.each(fn(handler) { handler() })
}

///|
/// Emit a double click event.
pub fn EventEmitter::emit_double_click(self : EventEmitter) -> Unit {
  self.double_click_handlers.each(fn(handler) { handler() })
}

///|
/// Emit a scroll event.
pub fn EventEmitter::emit_scroll(self : EventEmitter, x : Double, y : Double) -> Unit {
  self.scroll_handlers.each(fn(handler) { handler(x, y) })
}

///|
/// Get the total number of handlers.
pub fn EventEmitter::handler_count(self : EventEmitter) -> Int {
  self.click_handlers.length() +
  self.change_handlers.length() +
  self.submit_handlers.length() +
  self.input_handlers.length() +
  self.blur_handlers.length() +
  self.focus_handlers.length() +
  self.double_click_handlers.length() +
  self.scroll_handlers.length()
}

///|
/// Check if has any click handlers.
pub fn EventEmitter::has_click_handlers(self : EventEmitter) -> Bool {
  self.click_handlers.length() > 0
}

///|
/// Check if has any change handlers.
pub fn EventEmitter::has_change_handlers(self : EventEmitter) -> Bool {
  self.change_handlers.length() > 0
}

///|
/// Check if has any submit handlers.
pub fn EventEmitter::has_submit_handlers(self : EventEmitter) -> Bool {
  self.submit_handlers.length() > 0
}
