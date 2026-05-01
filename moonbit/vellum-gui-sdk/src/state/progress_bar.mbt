///|
/// ProgressBar represents a progress indicator.
pub struct ProgressBar {
  base : Widget
  value : Double
  max : Double
  is_indeterminate : Bool
}

///|
/// Create a new ProgressBar with a value.
pub fn ProgressBar::new(value : Double) -> ProgressBar {
  {
    base: Widget::new("progress-bar", "progress-gen-1"),
    value: value,
    max: 100.0,
    is_indeterminate: false,
  }
}

///|
/// Set the current value.
pub fn ProgressBar::with_value(self : ProgressBar, value : Double) -> ProgressBar {
  { ..self, value: value }
}

///|
/// Set the maximum value.
pub fn ProgressBar::with_max(self : ProgressBar, max : Double) -> ProgressBar {
  { ..self, max: max }
}

///|
/// Set whether the progress is indeterminate.
pub fn ProgressBar::with_indeterminate(self : ProgressBar, is_indeterminate : Bool) -> ProgressBar {
  { ..self, is_indeterminate: is_indeterminate }
}

///|
/// Get the current value.
pub fn ProgressBar::get_value(self : ProgressBar) -> Double {
  self.value
}

///|
/// Get the maximum value.
pub fn ProgressBar::get_max(self : ProgressBar) -> Double {
  self.max
}

///|
/// Check if the progress is indeterminate.
pub fn ProgressBar::is_indeterminate_progress(self : ProgressBar) -> Bool {
  self.is_indeterminate
}

///|
/// Get the progress as a ratio (0.0 to 1.0).
pub fn ProgressBar::get_progress(self : ProgressBar) -> Double {
  if self.max == 0.0 {
    0.0
  } else {
    self.value / self.max
  }
}

///|
/// Get the base widget.
pub fn ProgressBar::base(self : ProgressBar) -> Widget {
  self.base
}
