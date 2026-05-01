///|
/// EventEmitter manages event subscriptions and dispatching.
pub struct EventEmitter {
  listeners : @list.List[(String, String)]
}

///|
/// Create a new EventEmitter.
pub fn EventEmitter::new() -> EventEmitter {
  { listeners: @list.empty() }
}

///|
/// Subscribe to an event by name.
pub fn EventEmitter::on(
  self : EventEmitter,
  event_name : String,
  callback : String,
) -> EventEmitter {
  { listeners: self.listeners.add((event_name, callback)) }
}

///|
/// Unsubscribe from an event by name.
pub fn EventEmitter::off(
  self : EventEmitter,
  event_name : String,
) -> EventEmitter {
  let filtered = self.listeners.filter(
    fn(entry) {
      match entry {
        (name, _) => !(name == event_name)
      }
    }
  )
  { listeners: filtered }
}

///|
/// Emit an event by name.
pub fn EventEmitter::emit(self : EventEmitter, event_name : String) -> Unit {
  for listener in self.listeners {
    match listener {
      (name, callback) => {
        if name == event_name {
          ignore(callback)
        }
      }
    }
  }
}

///|
/// Get the number of listeners.
pub fn EventEmitter::listener_count(self : EventEmitter) -> Int {
  self.listeners.length()
}

///|
/// Check if has listeners for a specific event.
pub fn EventEmitter::has_listeners(self : EventEmitter, event_name : String) -> Bool {
  let count = self.listeners.filter(
    fn(entry) {
      match entry {
        (name, _) => name == event_name
      }
    }
  ).length()
  count > 0
}
