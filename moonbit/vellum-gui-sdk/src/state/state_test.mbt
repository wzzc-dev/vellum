///|
/// Black box tests for the State package.
test "observable new" {
  let obs = Observable::new(42)
  inspect(obs.get(), content="42")
}

test "state new" {
  let state = State::new("hello")
  inspect(state.get(), content="hello")
}

test "state set" {
  let state = State::new(0)
  ignore(state.set(42))
  inspect(state.get(), content="42")
}

test "state set same value returns false" {
  let state = State::new(42)
  inspect(state.set(42), content="false")
}

test "state update" {
  let state = State::new(2)
  state.update(fn(x) { x * 3 })
  inspect(state.get(), content="6")
}

test "binding from state" {
  let state = State::new("test")
  let binding = Binding::from_state(state)
  
  inspect(binding.get(), content="test")
  inspect(binding.set("updated"), content="true")
  inspect(state.get(), content="updated")
}

test "binding constant" {
  let binding = Binding::constant(999)
  inspect(binding.get(), content="999")
  inspect(binding.set(123), content="false")
  inspect(binding.get(), content="999")
}

test "computed basic" {
  let a = State::new(1)
  let b = State::new(2)
  
  let sum = computed(fn() -> Int { a.get() + b.get() })
  inspect(sum.get(), content="3")
}

test "computed caches value" {
  let state = State::new(10)
  let mut call_count = 0
  
  let doubled = computed(fn() -> Int {
    call_count = call_count + 1
    state.get() * 2
  })
  
  inspect(doubled.get(), content="20")
  inspect(doubled.get(), content="20")
  inspect(call_count, content="1")
}

test "computed mark_dirty forces recalculation" {
  let state = State::new(5)
  let doubled = computed(fn() -> Int { state.get() * 2 })
  
  inspect(doubled.get(), content="10")
  
  ignore(state.set(10))
  doubled.mark_dirty()
  
  inspect(doubled.get(), content="20")
}

test "autorun executes immediately" {
  let mut executed = false
  
  let _handle = autorun(fn() {
    executed = true
  })
  
  inspect(executed, content="true")
}

test "autorun can be disposed" {
  let mut call_count = 0
  
  let handle = autorun(fn() {
    call_count = call_count + 1
  })
  
  inspect(call_count, content="1")
  
  handle.dispose()
  inspect(handle.is_disposed(), content="true")
}

test "watch receives new and old values" {
  let state = State::new(10)
  let mut received_new = 0
  let mut received_old = 0
  
  let _handle = watch(state, fn(new_val, old_val) {
    received_new = new_val
    received_old = old_val
  })
  
  ignore(state.set(20))
  
  inspect(received_new, content="20")
  inspect(received_old, content="10")
}

test "watch can be disposed" {
  let state = State::new(0)
  let mut call_count = 0
  
  let handle = watch(state, fn(_, _) {
    call_count = call_count + 1
  })
  
  ignore(state.set(1))
  inspect(call_count, content="1")
  
  handle.dispose()
}

test "AutorunHandle new" {
  let mut disposed = false
  let handle = AutorunHandle::new(fn() {
    disposed = true
  })
  
  inspect(handle.is_disposed(), content="false")
  
  handle.dispose()
  inspect(handle.is_disposed(), content="true")
  inspect(disposed, content="true")
}

test "AutorunHandle dispose is idempotent" {
  let mut dispose_count = 0
  let handle = AutorunHandle::new(fn() {
    dispose_count = dispose_count + 1
  })
  
  handle.dispose()
  handle.dispose()
  handle.dispose()
  
  inspect(dispose_count, content="1")
}

test "WatchHandle new" {
  let handle = WatchHandle::new(fn() {}, 42)
  
  inspect(handle.is_disposed(), content="false")
  
  handle.dispose()
  inspect(handle.is_disposed(), content="true")
}

test "Computed::new with string" {
  let state = State::new("hello")
  let computed_val = computed(fn() -> String { state.get() + " world" })
  
  inspect(computed_val.get(), content="hello world")
}

test "multiple observers on same state" {
  let state = State::new(0)
  let mut a = 0
  let mut b = 0
  
  state.observe(fn(v) { a = v })
  state.observe(fn(v) { b = v * 2 })
  
  ignore(state.set(5))
  
  inspect(a, content="5")
  inspect(b, content="10")
}

test "state update with complex function" {
  let state = State::new(2)
  
  state.update(fn(x) { x + 10 })
  
  inspect(state.get(), content="12")
}
