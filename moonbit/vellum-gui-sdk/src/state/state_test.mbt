///|
/// Black box tests for the State package.
test "observable new" {
  let obs = new_observable(42)
  assert_eq(get_observable(&obs), 42)
}

test "state new" {
  let state = new_state("hello")
  assert_eq(get_state(&state), "hello")
}

test "state set" {
  let mut state = new_state(0)
  assert_eq(set_state(&mut state, 42), true)
  assert_eq(get_state(&state), 42)
}

test "state set same value returns false" {
  let mut state = new_state(42)
  assert_eq(set_state(&mut state, 42), false)
}

test "state update" {
  let mut state = new_state(2)
  update_state(&mut state, fn(x) { x * 3 })
  assert_eq(get_state(&state), 6)
}

test "binding from state" {
  let mut state = new_state("test")
  let binding = binding_from_state(&mut state)
  
  assert_eq(get_binding(&binding), "test")
  assert_eq(set_binding(&binding, "updated"), true)
  assert_eq(get_state(&state), "updated")
}

test "binding constant" {
  let binding = constant_binding(999)
  assert_eq(get_binding(&binding), 999)
  assert_eq(set_binding(&binding, 123), false)
  assert_eq(get_binding(&binding), 999)
}
