///|
/// Tests for Toggle.
test "toggle new" {
  let toggle = Toggle::new("dark-mode", "Dark Mode", false)
  assert_eq(toggle.get_label(), "Dark Mode")
  assert_eq(toggle.is_enabled(), false)
}

test "toggle new on" {
  let toggle = Toggle::new("dark-mode", "Dark Mode", true)
  assert_eq(toggle.is_enabled(), true)
}

test "toggle with disabled" {
  let toggle = Toggle::new("dark-mode", "Dark Mode", false).with_disabled(true)
  assert_eq(toggle.is_toggle_disabled(), true)
}

test "toggle with on" {
  let toggle = Toggle::new("dark-mode", "Dark Mode", false).with_on(true)
  assert_eq(toggle.is_enabled(), true)
}

test "toggle toggle" {
  let toggle = Toggle::new("dark-mode", "Dark Mode", false).toggle()
  assert_eq(toggle.is_enabled(), true)
}

test "toggle toggle twice" {
  let toggle = Toggle::new("dark-mode", "Dark Mode", true).toggle().toggle()
  assert_eq(toggle.is_enabled(), true)
}

test "toggle base widget" {
  let toggle = Toggle::new("dark-mode", "Dark Mode", false)
  let base = toggle.base()
  assert_eq(base.get_type(), "toggle")
}
