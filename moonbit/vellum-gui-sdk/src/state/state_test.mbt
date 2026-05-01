///|
/// Black box tests for the State package.
test "observable new" {
  let obs = Observable::new(42)
  assert_eq(obs.get(), 42)
}

test "state new" {
  let state = State::new("hello")
  assert_eq(state.get(), "hello")
}

test "state set" {
  let state = State::new(0)
  assert_eq(state.set(42), true)
  assert_eq(state.get(), 42)
}

test "state set same value returns false" {
  let state = State::new(42)
  assert_eq(state.set(42), false)
}

test "state update" {
  let state = State::new(2)
  state.update(fn(x) { x * 3 })
  assert_eq(state.get(), 6)
}

test "binding from state" {
  let state = State::new("test")
  let binding = Binding::from_state(state)
  
  assert_eq(binding.get(), "test")
  assert_eq(binding.set("updated"), true)
  assert_eq(state.get(), "updated")
}

test "binding constant" {
  let binding = Binding::constant(999)
  assert_eq(binding.get(), 999)
  assert_eq(binding.set(123), false)
  assert_eq(binding.get(), 999)
}
