///|
/// SelectOption represents an option in a Select dropdown.
pub struct SelectOption {
  value : String
  label : String
}

///|
/// Create a new SelectOption.
pub fn SelectOption::new(value : String, label : String) -> SelectOption {
  { value: value, label: label }
}

///|
/// Get the value.
pub fn SelectOption::get_value(self : SelectOption) -> String {
  self.value
}

///|
/// Get the label.
pub fn SelectOption::get_label(self : SelectOption) -> String {
  self.label
}

///|
/// Select represents a dropdown select input.
pub struct Select {
  base : Widget
  options : @list.List[SelectOption]
  selected_value : String
  is_disabled : Bool
  is_open : Bool
}

///|
/// Create a new Select with an ID and options.
pub fn Select::new(id : String, options : @list.List[SelectOption]) -> Select {
  {
    base: Widget::new("select", id),
    options: options,
    selected_value: "",
    is_disabled: false,
    is_open: false,
  }
}

///|
/// Set the selected value.
pub fn Select::with_selected(self : Select, value : String) -> Select {
  { ..self, selected_value: value }
}

///|
/// Set whether the select is disabled.
pub fn Select::with_disabled(self : Select, is_disabled : Bool) -> Select {
  { ..self, is_disabled: is_disabled }
}

///|
/// Set whether the dropdown is open.
pub fn Select::with_open(self : Select, is_open : Bool) -> Select {
  { ..self, is_open: is_open }
}

///|
/// Get the selected value.
pub fn Select::get_selected(self : Select) -> String {
  self.selected_value
}

///|
/// Check if the select is disabled.
pub fn Select::is_select_disabled(self : Select) -> Bool {
  self.is_disabled
}

///|
/// Check if the dropdown is open.
pub fn Select::is_dropdown_open(self : Select) -> Bool {
  self.is_open
}

///|
/// Get the number of options.
pub fn Select::option_count(self : Select) -> Int {
  self.options.length()
}

///|
/// Toggle the dropdown.
pub fn Select::toggle_dropdown(self : Select) -> Select {
  { ..self, is_open: !self.is_open }
}

///|
/// Get the base widget.
pub fn Select::base(self : Select) -> Widget {
  self.base
}
