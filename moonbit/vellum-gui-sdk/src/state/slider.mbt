///|
/// Slider represents a slider input.
pub struct Slider {
  base : Widget
  value : Double
  min : Double
  max : Double
  step : Double
  is_disabled : Bool
}

///|
/// Create a new Slider with an ID, value, min, and max.
pub fn Slider::new(id : String, value : Double, min : Double, max : Double) -> Slider {
  {
    base: Widget::new("slider", id),
    value: value,
    min: min,
    max: max,
    step: 1.0,
    is_disabled: false,
  }
}

///|
/// Set the current value.
pub fn Slider::with_value(self : Slider, value : Double) -> Slider {
  { ..self, value: value }
}

///|
/// Set the step size.
pub fn Slider::with_step(self : Slider, step : Double) -> Slider {
  { ..self, step: step }
}

///|
/// Set whether the slider is disabled.
pub fn Slider::with_disabled(self : Slider, is_disabled : Bool) -> Slider {
  { ..self, is_disabled: is_disabled }
}

///|
/// Get the current value.
pub fn Slider::get_value(self : Slider) -> Double {
  self.value
}

///|
/// Get the minimum value.
pub fn Slider::get_min(self : Slider) -> Double {
  self.min
}

///|
/// Get the maximum value.
pub fn Slider::get_max(self : Slider) -> Double {
  self.max
}

///|
/// Get the step size.
pub fn Slider::get_step(self : Slider) -> Double {
  self.step
}

///|
/// Check if the slider is disabled.
pub fn Slider::is_slider_disabled(self : Slider) -> Bool {
  self.is_disabled
}

///|
/// Get the progress as a ratio (0.0 to 1.0).
pub fn Slider::get_progress(self : Slider) -> Double {
  if self.max == self.min {
    0.0
  } else {
    (self.value - self.min) / (self.max - self.min)
  }
}

///|
/// Get the base widget.
pub fn Slider::base(self : Slider) -> Widget {
  self.base
}
