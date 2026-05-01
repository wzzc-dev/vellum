///|
/// Toggle represents a toggle switch.
pub struct Toggle {
  base : Widget
  label : String
  is_on : Bool
  is_disabled : Bool
}

///|
/// Create a new Toggle with an ID, label, and initial state.
pub fn Toggle::new(id : String, label : String, is_on : Bool) -> Toggle {
  {
    base: Widget::new("toggle", id),
    label: label,
    is_on: is_on,
    is_disabled: false,
  }
}

///|
/// Set whether the toggle is disabled.
pub fn Toggle::with_disabled(self : Toggle, is_disabled : Bool) -> Toggle {
  { ..self, is_disabled: is_disabled }
}

///|
/// Set the on/off state.
pub fn Toggle::with_on(self : Toggle, is_on : Bool) -> Toggle {
  { ..self, is_on: is_on }
}

///|
/// Check if the toggle is on.
pub fn Toggle::is_enabled(self : Toggle) -> Bool {
  self.is_on
}

///|
/// Get the label.
pub fn Toggle::get_label(self : Toggle) -> String {
  self.label
}

///|
/// Check if the toggle is disabled.
pub fn Toggle::is_toggle_disabled(self : Toggle) -> Bool {
  self.is_disabled
}

///|
/// Toggle the state.
pub fn Toggle::toggle(self : Toggle) -> Toggle {
  { ..self, is_on: !self.is_on }
}

///|
/// Get the base widget.
pub fn Toggle::base(self : Toggle) -> Widget {
  self.base
}
