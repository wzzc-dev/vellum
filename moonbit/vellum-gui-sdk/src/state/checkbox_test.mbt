///|
/// Tests for Checkbox.
test "checkbox new" {
  let cb = Checkbox::new("agree", "I agree", false)
  assert_eq(cb.get_label(), "I agree")
  assert_eq(cb.is_checked(), false)
}

test "checkbox new checked" {
  let cb = Checkbox::new("agree", "I agree", true)
  assert_eq(cb.is_checked(), true)
}

test "checkbox with disabled" {
  let cb = Checkbox::new("agree", "I agree", false).with_disabled(true)
  assert_eq(cb.is_checkbox_disabled(), true)
}

test "checkbox with checked" {
  let cb = Checkbox::new("agree", "I agree", false).with_checked(true)
  assert_eq(cb.is_checked(), true)
}

test "checkbox toggle" {
  let cb = Checkbox::new("agree", "I agree", false).toggle()
  assert_eq(cb.is_checked(), true)
}

test "checkbox toggle twice" {
  let cb = Checkbox::new("agree", "I agree", true).toggle().toggle()
  assert_eq(cb.is_checked(), true)
}

test "checkbox base widget" {
  let cb = Checkbox::new("agree", "I agree", false)
  let base = cb.base()
  assert_eq(base.get_type(), "checkbox")
}
