///|
/// Observable[T]: A value that can be observed for changes.
pub struct Observable[T] {
  mut value : T
  mut observers : @list.List[(T) -> Unit]
}

///|
/// State[T]: A wrapper around Observable providing convenient state management.
pub struct State[T] {
  observable : Observable[T]
}

///|
/// Binding[T]: A bidirectional binding to a value, providing get and set methods.
pub struct Binding[T] {
  get : () -> T
  set : (T) -> Bool
}

///|
/// Computed[T]: A computed value that automatically updates when dependencies change.
pub struct Computed[T] {
  getter : () -> T
  mut cached_value : T
  mut dirty : Bool
}

///|
/// AutorunHandle: A handle to an autorun effect that can be disposed.
pub struct AutorunHandle {
  mut dispose_fn : () -> Unit
  mut is_disposed : Bool
}

///|
/// WatchHandle: A handle to a watcher that can be disposed.
pub struct WatchHandle[T] {
  mut dispose_fn : () -> Unit
  mut is_disposed : Bool
  mut last_value : T
}
