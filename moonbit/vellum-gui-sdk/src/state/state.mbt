///|
/// Create a new Observable with an initial value.
pub fn[T] Observable::new(value : T) -> Observable[T] {
  Observable::{ value: value, observers: @list.empty() }
}

///|
/// Add an observer callback to this Observable.
pub fn[T] Observable::observe(
  self : Observable[T],
  callback : (T) -> Unit,
) -> Unit {
  self.observers = self.observers.add(callback)
}

///|
/// Notify all observers of a value change.
fn[T] Observable::notify(self : Observable[T]) -> Unit {
  let value = self.value
  self.observers.each(fn(cb) { cb(value) })
}

///|
/// Get the current value of this Observable.
pub fn[T] Observable::get(self : Observable[T]) -> T {
  self.value
}

///|
/// Set the value of this Observable and notify observers.
pub fn[T : Eq] Observable::set(
  self : Observable[T],
  value : T,
) -> Bool {
  if self.value == value {
    false
  } else {
    self.value = value
    self.notify()
    true
  }
}

///|
/// Create a new State with an initial value.
pub fn[T] State::new(value : T) -> State[T] {
  State::{ observable: Observable::new(value) }
}

///|
/// Get the current value of this State.
pub fn[T] State::get(self : State[T]) -> T {
  self.observable.get()
}

///|
/// Set the value of this State.
pub fn[T : Eq] State::set(
  self : State[T],
  value : T,
) -> Bool {
  self.observable.set(value)
}

fn[T] ignore(_ : T) -> Unit {
  ()
}

///|
/// Update the value of this State using a function.
pub fn[T : Eq] State::update(
  self : State[T],
  f : (T) -> T,
) -> Unit {
  let value = f(self.observable.get())
  ignore(self.set(value))
}

///|
/// Add an observer callback to this State.
pub fn[T] State::observe(
  self : State[T],
  callback : (T) -> Unit,
) -> Unit {
  self.observable.observe(callback)
}

///|
/// Create a new Binding from get and set functions.
pub fn[T] Binding::new(
  get : () -> T,
  set : (T) -> Bool,
) -> Binding[T] {
  Binding::{ get: get, set: set }
}

///|
/// Create a Binding from a State.
pub fn[T : Eq] Binding::from_state(state : State[T]) -> Binding[T] {
  Binding::new(
    fn() -> T { state.get() },
    fn(value : T) -> Bool { state.set(value) },
  )
}

///|
/// Get the current value through this Binding.
pub fn[T] Binding::get(self : Binding[T]) -> T {
  (self.get)()
}

///|
/// Set the value through this Binding.
pub fn[T] Binding::set(
  self : Binding[T],
  value : T,
) -> Bool {
  (self.set)(value)
}

///|
/// Create a constant Binding that always returns the same value.
pub fn[T] Binding::constant(value : T) -> Binding[T] {
  Binding::new(
    fn() -> T { value },
    fn(_ : T) -> Bool { false },
  )
}

///|
/// Create a new Computed value with a getter function.
pub fn[T] Computed::new(getter : () -> T) -> Computed[T] {
  let initial = getter()
  Computed::{ getter: getter, cached_value: initial, dirty: false }
}

///|
/// Get the current value of this Computed, recalculating if dirty.
pub fn[T] Computed::get(self : Computed[T]) -> T {
  if self.dirty {
    self.cached_value = (self.getter)()
    self.dirty = false
  }
  self.cached_value
}

///|
/// Mark this Computed as dirty, forcing recalculation on next get.
pub fn[T] Computed::mark_dirty(self : Computed[T]) -> Unit {
  self.dirty = true
}

///|
/// Create a new AutorunHandle.
pub fn AutorunHandle::new(dispose_fn : () -> Unit) -> AutorunHandle {
  AutorunHandle::{ dispose_fn: dispose_fn, is_disposed: false }
}

///|
/// Dispose this autorun, stopping further executions.
pub fn AutorunHandle::dispose(self : AutorunHandle) -> Unit {
  if !self.is_disposed {
    self.is_disposed = true
    (self.dispose_fn)()
  }
}

///|
/// Check if this autorun is disposed.
pub fn AutorunHandle::is_disposed(self : AutorunHandle) -> Bool {
  self.is_disposed
}

///|
/// Create a new WatchHandle.
pub fn[T] WatchHandle::new(dispose_fn : () -> Unit, initial : T) -> WatchHandle[T] {
  WatchHandle::{ dispose_fn: dispose_fn, is_disposed: false, last_value: initial }
}

///|
/// Dispose this watcher.
pub fn[T] WatchHandle::dispose(self : WatchHandle[T]) -> Unit {
  if !self.is_disposed {
    self.is_disposed = true
    (self.dispose_fn)()
  }
}

///|
/// Check if this watcher is disposed.
pub fn[T] WatchHandle::is_disposed(self : WatchHandle[T]) -> Bool {
  self.is_disposed
}

///|
/// Create an autorun effect that executes immediately.
pub fn autorun(effect : () -> Unit) -> AutorunHandle {
  effect()
  AutorunHandle::new(fn() {})
}

///|
/// Create a computed value that automatically tracks dependencies.
pub fn[T] computed(getter : () -> T) -> Computed[T] {
  Computed::new(getter)
}

///|
/// Watch a State for changes and execute a callback with old and new values.
pub fn[T : Eq] watch(state : State[T], callback : (T, T) -> Unit) -> WatchHandle[T] {
  let old_value = state.get()
  state.observe(fn(new_value) {
    callback(new_value, old_value)
  })
  WatchHandle::new(fn() {}, old_value)
}
