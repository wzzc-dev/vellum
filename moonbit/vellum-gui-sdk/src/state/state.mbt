///|
/// Create a new Observable with an initial value.
pub fn[T] Observable::new(value : T) -> Observable[T] {
  { value: value, observers: @list.empty() }
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
  { observable: Observable::new(value) }
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

fn ignore[T](_ : T) -> Unit {
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
  { get: get, set: set }
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
