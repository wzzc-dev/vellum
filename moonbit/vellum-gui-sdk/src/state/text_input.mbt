///|
/// TextInput represents a text input field.
pub struct TextInput {
  base : Widget
  value : String
  placeholder : String
  is_password : Bool
  is_disabled : Bool
  max_length : Int
}

///|
/// Create a new TextInput with an ID and placeholder.
pub fn TextInput::new(id : String, placeholder : String) -> TextInput {
  {
    base: Widget::new("text-input", id),
    value: "",
    placeholder: placeholder,
    is_password: false,
    is_disabled: false,
    max_length: 0,
  }
}

///|
/// Set the current value.
pub fn TextInput::with_value(self : TextInput, value : String) -> TextInput {
  { ..self, value: value }
}

///|
/// Set whether this is a password field.
pub fn TextInput::with_password(self : TextInput, is_password : Bool) -> TextInput {
  { ..self, is_password: is_password }
}

///|
/// Set whether this input is disabled.
pub fn TextInput::with_disabled(self : TextInput, is_disabled : Bool) -> TextInput {
  { ..self, is_disabled: is_disabled }
}

///|
/// Set the maximum length of input.
pub fn TextInput::with_max_length(self : TextInput, max : Int) -> TextInput {
  { ..self, max_length: max }
}

///|
/// Get the current value.
pub fn TextInput::get_value(self : TextInput) -> String {
  self.value
}

///|
/// Get the placeholder text.
pub fn TextInput::get_placeholder(self : TextInput) -> String {
  self.placeholder
}

///|
/// Check if this is a password field.
pub fn TextInput::is_password_field(self : TextInput) -> Bool {
  self.is_password
}

///|
/// Check if this input is disabled.
pub fn TextInput::is_input_disabled(self : TextInput) -> Bool {
  self.is_disabled
}

///|
/// Get the max length (0 means unlimited).
pub fn TextInput::get_max_length(self : TextInput) -> Int {
  self.max_length
}

///|
/// Get the base widget.
pub fn TextInput::base(self : TextInput) -> Widget {
  self.base
}
