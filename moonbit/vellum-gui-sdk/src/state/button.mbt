///|
/// Button types.
pub enum ButtonType {
  Normal
  Icon
  Text
}

///|
/// Button widget for user interaction.
pub struct Button {
  base : Widget
  label : String
  button_type : ButtonType
  disabled : Bool
}

///|
/// Create a new Button with the given label.
pub fn Button::new(label : String) -> Button {
  {
    base: Widget::new("button", "button-gen-1"),
    label: label,
    button_type: Normal,
    disabled: false,
  }
}

///|
/// Create an icon button.
pub fn Button::icon(icon_name : String) -> Button {
  {
    base: Widget::new("icon-button", "button-gen-1"),
    label: icon_name,
    button_type: Icon,
    disabled: false,
  }
}

///|
/// Create a text button.
pub fn Button::text(label : String) -> Button {
  {
    base: Widget::new("text-button", "button-gen-1"),
    label: label,
    button_type: Text,
    disabled: false,
  }
}

///|
/// Set the button as disabled.
pub fn Button::with_disabled(self : Button, disabled : Bool) -> Button {
  { ..self, disabled: disabled }
}

///|
/// Get the base widget.
pub fn Button::base(self : Button) -> Widget {
  self.base
}

///|
/// Get the button label.
pub fn Button::get_label(self : Button) -> String {
  self.label
}

///|
/// Get the button type.
pub fn Button::get_type(self : Button) -> ButtonType {
  self.button_type
}

///|
/// Check if the button is disabled.
pub fn Button::is_disabled(self : Button) -> Bool {
  self.disabled
}

///|
/// Subscribe to click events.
pub fn Button::on_click(self : Button, handler : () -> Unit) -> Button {
  { ..self, base: self.base.on_click(handler) }
}

///|
/// Emit a click event.
pub fn Button::emit_click(self : Button) -> Unit {
  self.base.emit_click()
}

///|
/// Set the button label.
pub fn Button::with_label(self : Button, label : String) -> Button {
  { ..self, label: label }
}
