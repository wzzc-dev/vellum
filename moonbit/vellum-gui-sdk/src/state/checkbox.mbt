///|
/// Checkbox represents a checkbox input.
pub struct Checkbox {
  base : Widget
  label : String
  checked : Bool
  is_disabled : Bool
}

///|
/// Create a new Checkbox with an ID, label, and initial checked state.
pub fn Checkbox::new(id : String, label : String, checked : Bool) -> Checkbox {
  Checkbox::{
    base: Widget::new("checkbox", id),
    label: label,
    checked: checked,
    is_disabled: false,
  }
}

///|
/// Set whether the checkbox is disabled.
pub fn Checkbox::with_disabled(self : Checkbox, is_disabled : Bool) -> Checkbox {
  { ..self, is_disabled: is_disabled }
}

///|
/// Set the checked state.
pub fn Checkbox::with_checked(self : Checkbox, checked : Bool) -> Checkbox {
  { ..self, checked: checked }
}

///|
/// Check if the checkbox is checked.
pub fn Checkbox::is_checked(self : Checkbox) -> Bool {
  self.checked
}

///|
/// Get the label.
pub fn Checkbox::get_label(self : Checkbox) -> String {
  self.label
}

///|
/// Check if the checkbox is disabled.
pub fn Checkbox::is_checkbox_disabled(self : Checkbox) -> Bool {
  self.is_disabled
}

///|
/// Toggle the checked state.
pub fn Checkbox::toggle(self : Checkbox) -> Checkbox {
  { ..self, checked: !self.checked }
}

///|
/// Get the base widget.
pub fn Checkbox::base(self : Checkbox) -> Widget {
  self.base
}
