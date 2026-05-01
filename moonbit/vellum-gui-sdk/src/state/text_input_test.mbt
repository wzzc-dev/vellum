///|
/// Tests for TextInput.
test "text input new" {
  let input = TextInput::new("name", "Enter your name")
  assert_eq(input.get_value(), "")
  assert_eq(input.get_placeholder(), "Enter your name")
  assert_eq(input.is_password_field(), false)
  assert_eq(input.is_input_disabled(), false)
}

test "text input with value" {
  let input = TextInput::new("name", "Enter name").with_value("John")
  assert_eq(input.get_value(), "John")
}

test "text input with password" {
  let input = TextInput::new("password", "Password").with_password(true)
  assert_eq(input.is_password_field(), true)
}

test "text input with disabled" {
  let input = TextInput::new("name", "Name").with_disabled(true)
  assert_eq(input.is_input_disabled(), true)
}

test "text input with max length" {
  let input = TextInput::new("name", "Name").with_max_length(50)
  assert_eq(input.get_max_length(), 50)
}

test "text input chaining" {
  let input = TextInput::new("password", "Enter password")
    .with_value("secret")
    .with_password(true)
    .with_max_length(20)
  assert_eq(input.get_value(), "secret")
  assert_eq(input.is_password_field(), true)
  assert_eq(input.get_max_length(), 20)
}

test "text input base widget" {
  let input = TextInput::new("name", "Name")
  let base = input.base()
  assert_eq(base.get_type(), "text-input")
}

test "text input default max length" {
  let input = TextInput::new("name", "Name")
  assert_eq(input.get_max_length(), 0)
}
