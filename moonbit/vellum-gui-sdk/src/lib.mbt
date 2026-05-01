///|
/// Vellum GUI Framework
/// A declarative UI framework with reactive state management.

///|
/// Re-export types and functions from the state package.
pub using @state {
  type Observable,
  type State,
  type Binding,
}

///|
pub using @state {
  fn new_observable,
  fn observe_observable,
  fn get_observable,
  fn set_observable,
  fn new_state,
  fn get_state,
  fn set_state,
  fn update_state,
  fn observe_state,
  fn new_binding,
  fn binding_from_state,
  fn get_binding,
  fn set_binding,
  fn constant_binding,
}
