///|
/// Comprehensive tests for EventEmitter.
test "event emitter creation" {
  let emitter = EventEmitter::new()
  assert_eq(emitter.handler_count(), 0)
}

test "event emitter on_click" {
  let mut clicked = false
  let emitter = EventEmitter::new()
    .on_click(fn() { clicked = true })
  
  assert(emitter.has_click_handlers())
  assert_eq(emitter.handler_count(), 1)
}

test "event emitter on_change" {
  let mut value = ""
  let emitter = EventEmitter::new()
    .on_change(fn(v) { value = v })
  
  assert(emitter.has_change_handlers())
}

test "event emitter on_submit" {
  let mut submitted = ""
  let emitter = EventEmitter::new()
    .on_submit(fn(v) { submitted = v })
  
  assert(emitter.has_submit_handlers())
}

test "event emitter emit_click" {
  let mut click_count = 0
  let emitter = EventEmitter::new()
    .on_click(fn() { click_count = click_count + 1 })
  
  emitter.emit_click()
  assert_eq(click_count, 1)
  
  emitter.emit_click()
  assert_eq(click_count, 2)
}

test "event emitter emit_change" {
  let mut received = ""
  let emitter = EventEmitter::new()
    .on_change(fn(v) { received = v })
  
  emitter.emit_change("hello")
  assert_eq(received, "hello")
}

test "event emitter emit_submit" {
  let mut submitted = ""
  let emitter = EventEmitter::new()
    .on_submit(fn(v) { submitted = v })
  
  emitter.emit_submit("test value")
  assert_eq(submitted, "test value")
}

test "event emitter multiple handlers" {
  let mut order = ""
  let emitter = EventEmitter::new()
    .on_click(fn() { order = order + "a" })
    .on_click(fn() { order = order + "b" })
  
  emitter.emit_click()
  assert_eq(order, "ab")
}

test "event emitter multiple event types" {
  let mut results = ""
  let emitter = EventEmitter::new()
    .on_click(fn() { results = results + "c" })
    .on_blur(fn() { results = results + "b" })
    .on_focus(fn() { results = results + "f" })
  
  emitter.emit_click()
  emitter.emit_focus()
  emitter.emit_blur()
  
  assert_eq(results, "cfb")
}

test "event emitter emit_blur" {
  let mut blurred = false
  let emitter = EventEmitter::new()
    .on_blur(fn() { blurred = true })
  
  emitter.emit_blur()
  assert(blurred)
}

test "event emitter emit_focus" {
  let mut focused = false
  let emitter = EventEmitter::new()
    .on_focus(fn() { focused = true })
  
  emitter.emit_focus()
  assert(focused)
}

test "event emitter emit_double_click" {
  let mut double_clicked = false
  let emitter = EventEmitter::new()
    .on_double_click(fn() { double_clicked = true })
  
  emitter.emit_double_click()
  assert(double_clicked)
}

test "event emitter emit_input" {
  let mut input_value = ""
  let emitter = EventEmitter::new()
    .on_input(fn(v) { input_value = v })
  
  emitter.emit_input("typed text")
  assert_eq(input_value, "typed text")
}

test "event emitter emit_scroll" {
  let mut scroll_x = 0.0
  let mut scroll_y = 0.0
  let emitter = EventEmitter::new()
    .on_scroll(fn(x, y) {
      scroll_x = x
      scroll_y = y
    })
  
  emitter.emit_scroll(10.0, 20.0)
  assert_eq(scroll_x, 10.0)
  assert_eq(scroll_y, 20.0)
}

test "event emitter all handlers count" {
  let emitter = EventEmitter::new()
    .on_click(fn() {})
    .on_change(fn(_) {})
    .on_submit(fn(_) {})
    .on_blur(fn() {})
  
  assert_eq(emitter.handler_count(), 4)
}
