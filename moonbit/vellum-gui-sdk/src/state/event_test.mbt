///|
/// Comprehensive tests for EventEmitter.
test "event emitter creation" {
  let emitter = EventEmitter::new()
  assert_eq(emitter.listener_count(), 0)
}

test "event emitter on subscribe" {
  let emitter = EventEmitter::new()
    .on("click", "callback1")
  assert_eq(emitter.listener_count(), 1)
}

test "event emitter multiple subscriptions" {
  let emitter = EventEmitter::new()
    .on("focus", "callback1")
    .on("blur", "callback2")
    .on("change", "callback3")
  assert_eq(emitter.listener_count(), 3)
}

test "event emitter has listeners" {
  let emitter = EventEmitter::new().on("focus", "callback1")
  assert_eq(emitter.has_listeners("focus"), true)
  assert_eq(emitter.has_listeners("blur"), false)
}

test "event emitter off unsubscribe" {
  let emitter = EventEmitter::new()
    .on("focus", "callback1")
    .off("focus")
  assert_eq(emitter.listener_count(), 0)
  assert_eq(emitter.has_listeners("focus"), false)
}

test "event emitter preserve other listeners on unsubscribe" {
  let emitter = EventEmitter::new()
    .on("focus", "callback1")
    .on("blur", "callback2")
    .on("change", "callback3")
    .off("focus")
  assert_eq(emitter.listener_count(), 2)
  assert_eq(emitter.has_listeners("blur"), true)
  assert_eq(emitter.has_listeners("change"), true)
}

test "event emitter emit does not crash" {
  let emitter = EventEmitter::new()
    .on("click", "callback1")
  emitter.emit("click")
  emitter.emit("focus")
}

test "event emitter empty emit" {
  let emitter = EventEmitter::new()
  emitter.emit("focus")
  emitter.emit("blur")
}

test "event emitter multiple listeners same type" {
  let emitter = EventEmitter::new()
    .on("focus", "callback1")
    .on("focus", "callback2")
  assert_eq(emitter.listener_count(), 2)
  assert_eq(emitter.has_listeners("focus"), true)
}

test "event emitter unsubscribing non-existent listener" {
  let emitter = EventEmitter::new()
    .on("blur", "callback1")
    .off("focus")
  assert_eq(emitter.listener_count(), 1)
}
