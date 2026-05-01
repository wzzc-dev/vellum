///|
/// Create a new Observable with an initial value.
pub fn[T] new_observable(value: T) -> Observable[T] {
  { value: value, observers: [] }
}

///|
/// Add an observer callback to this Observable.
pub fn[T] observe_observable(self: &mut Observable[T], callback: (T) -> Unit) -> Unit {
  self.observers.push(callback)
}

///|
/// Notify all observers of a value change.
fn[T] notify_observable(self: &mut Observable[T]) -> Unit {
  let value = self.value
  for callback in self.observers {
    callback(value)
  }
}

///|
/// Get the current value of this Observable.
pub fn[T] get_observable(self: &Observable[T]) -> T {
  self.value
}

///|
/// Set the value of this Observable and notify observers.
pub fn[T] set_observable(self: &mut Observable[T], value: T) -> Bool {
  if self.value == value {
    false
  } else {
    self.value = value
    notify_observable(self)
    true
  }
}

///|
/// Create a new State with an initial value.
pub fn[T] new_state(value: T) -> State[T] {
  { observable: new_observable(value) }
}

///|
/// Get the current value of this State.
pub fn[T] get_state(self: &State[T]) -> T {
  get_observable(&self.observable)
}

///|
/// Set the value of this State.
pub fn[T] set_state(self: &mut State[T], value: T) -> Bool {
  set_observable(&mut self.observable, value)
}

///|
/// Update the value of this State using a function.
pub fn[T] update_state(self: &mut State[T], f: (T) -> T) -> Unit {
  let value = f(get_observable(&self.observable))
  set_state(self, value)
}

///|
/// Add an observer callback to this State.
pub fn[T] observe_state(self: &mut State[T], callback: (T) -> Unit) -> Unit {
  observe_observable(&mut self.observable, callback)
}

///|
/// Create a new Binding from get and set functions.
pub fn[T] new_binding(get: () -> T, set: (T) -> Bool) -> Binding[T] {
  { get: get, set: set }
}

///|
/// Create a Binding from a State.
pub fn[T] binding_from_state(state: &mut State[T]) -> Binding[T] {
  new_binding(
    fn() { get_state(state) },
    fn(value) { set_state(state, value) }
  )
}

///|
/// Get the current value through this Binding.
pub fn[T] get_binding(self: &Binding[T]) -> T {
  (self.get)()
}

///|
/// Set the value through this Binding.
pub fn[T] set_binding(self: &Binding[T], value: T) -> Bool {
  (self.set)(value)
}

///|
/// Create a constant Binding that always returns the same value.
pub fn[T] constant_binding(value: T) -> Binding[T] {
  new_binding(
    fn() { value },
    fn(_) { false }
  )
}
