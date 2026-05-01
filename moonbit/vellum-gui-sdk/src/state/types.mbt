///|
/// Observable[T]: A value that can be observed for changes.
pub(all) struct Observable[T] {
  mut value : T
  mut observers : @list.List[(T) -> Unit]
}

///|
/// State[T]: A wrapper around Observable providing convenient state management.
pub(all) struct State[T] {
  observable : Observable[T]
}

///|
/// Binding[T]: A bidirectional binding to a value, providing get and set methods.
pub(all) struct Binding[T] {
  get : () -> T
  set : (T) -> Bool
}
