///|
/// Tests for TextInput.
test "text input new" {
  let input = TextInput::new("name", "Enter your name")
  inspect(input.get_value(), content="")
  inspect(input.get_placeholder(), content="Enter your name")
  inspect(input.is_password_field(), content="false")
  inspect(input.is_input_disabled(), content="false")
}

test "text input with value" {
  let input = TextInput::new("name", "Enter name").with_value("John")
  inspect(input.get_value(), content="John")
}

test "text input with password" {
  let input = TextInput::new("password", "Password").with_password(true)
  inspect(input.is_password_field(), content="true")
}

test "text input with disabled" {
  let input = TextInput::new("name", "Name").with_disabled(true)
  inspect(input.is_input_disabled(), content="true")
}

test "text input with max length" {
  let input = TextInput::new("name", "Name").with_max_length(50)
  inspect(input.get_max_length(), content="50")
}

test "text input chaining" {
  let input = TextInput::new("password", "Enter password")
    .with_value("secret")
    .with_password(true)
    .with_max_length(20)
  inspect(input.get_value(), content="secret")
  inspect(input.is_password_field(), content="true")
  inspect(input.get_max_length(), content="20")
}

test "text input base widget" {
  let input = TextInput::new("name", "Name")
  let base = input.base()
  inspect(base.get_type(), content="text-input")
}

test "text input default max length" {
  let input = TextInput::new("name", "Name")
  inspect(input.get_max_length(), content="0")
}

test "text input auto_focus" {
  let input = TextInput::new("name", "Name").auto_focus(true)
  inspect(input.is_auto_focus(), content="true")
}

test "text input on_input event" {
  let input = TextInput::new("name", "Name")
    .on_input(fn(value) { () })
  let base = input.base()
  inspect(base.get_type(), content="text-input")
}

test "text input on_submit event" {
  let input = TextInput::new("name", "Name")
    .on_submit(fn(value) { () })
  inspect(input.get_value(), content="")
}

test "text input on_blur event" {
  let input = TextInput::new("name", "Name")
    .on_blur(fn() { () })
  inspect(input.get_placeholder(), content="Name")
}

test "text input on_focus event" {
  let input = TextInput::new("name", "Name")
    .on_focus(fn() { () })
  inspect(input.get_value(), content="")
}

test "text input multiple event handlers" {
  let input = TextInput::new("name", "Name")
    .on_input(fn(v) { () })
    .on_submit(fn(v) { () })
    .on_blur(fn() { () })
    .auto_focus(true)
  inspect(input.is_auto_focus(), content="true")
}
