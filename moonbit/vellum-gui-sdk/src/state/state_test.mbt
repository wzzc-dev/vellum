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

test "computed basic" {
  let a = State::new(1)
  let b = State::new(2)
  
  let sum = computed(fn() -> Int { a.get() + b.get() })
  assert_eq(sum.get(), 3)
}

test "computed caches value" {
  let state = State::new(10)
  let mut call_count = 0
  
  let doubled = computed(fn() -> Int {
    call_count = call_count + 1
    state.get() * 2
  })
  
  assert_eq(doubled.get(), 20)
  assert_eq(doubled.get(), 20)
  assert_eq(call_count, 1)
}

test "computed mark_dirty forces recalculation" {
  let state = State::new(5)
  let doubled = computed(fn() -> Int { state.get() * 2 })
  
  assert_eq(doubled.get(), 10)
  
  state.set(10)
  doubled.mark_dirty()
  
  assert_eq(doubled.get(), 20)
}

test "autorun executes immediately" {
  let mut executed = false
  
  let _handle = autorun(fn() {
    executed = true
  })
  
  assert(executed)
}

test "autorun can be disposed" {
  let mut call_count = 0
  
  let handle = autorun(fn() {
    call_count = call_count + 1
  })
  
  assert_eq(call_count, 1)
  
  handle.dispose()
  assert(handle.is_disposed())
}

test "watch receives new and old values" {
  let state = State::new(10)
  let mut received_new = 0
  let mut received_old = 0
  
  let _handle = watch(state, fn(new_val, old_val) {
    received_new = new_val
    received_old = old_val
  })
  
  state.set(20)
  
  assert_eq(received_new, 20)
  assert_eq(received_old, 10)
}

test "watch can be disposed" {
  let state = State::new(0)
  let mut call_count = 0
  
  let handle = watch(state, fn(_, _) {
    call_count = call_count + 1
  })
  
  state.set(1)
  assert_eq(call_count, 1)
  
  handle.dispose()
  state.set(2)
  assert_eq(call_count, 1)
}

test "AutorunHandle new" {
  let mut disposed = false
  let handle = AutorunHandle::new(fn() {
    disposed = true
  })
  
  assert_eq(handle.is_disposed(), false)
  
  handle.dispose()
  assert(handle.is_disposed())
  assert(disposed)
}

test "AutorunHandle dispose is idempotent" {
  let mut dispose_count = 0
  let handle = AutorunHandle::new(fn() {
    dispose_count = dispose_count + 1
  })
  
  handle.dispose()
  handle.dispose()
  handle.dispose()
  
  assert_eq(dispose_count, 1)
}

test "WatchHandle new" {
  let handle = WatchHandle::new(fn() {}, 42)
  
  assert_eq(handle.is_disposed(), false)
  
  handle.dispose()
  assert(handle.is_disposed())
}

test "Computed::new with string" {
  let state = State::new("hello")
  let computed_val = computed(fn() -> String { state.get() + " world" })
  
  assert_eq(computed_val.get(), "hello world")
}

test "multiple observers on same state" {
  let state = State::new(0)
  let mut a = 0
  let mut b = 0
  
  state.observe(fn(v) { a = v })
  state.observe(fn(v) { b = v * 2 })
  
  state.set(5)
  
  assert_eq(a, 5)
  assert_eq(b, 10)
}

test "state update with complex function" {
  let state = State::new([1, 2, 3])
  
  state.update(fn(arr) { arr.add(4) })
  
  assert_eq(state.get().length(), 4)
}
