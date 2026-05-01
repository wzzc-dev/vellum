///|
/// Tests for Slider.
test "slider new" {
  let slider = Slider::new("volume", 50.0, 0.0, 100.0)
  assert_eq(slider.get_value(), 50.0)
  assert_eq(slider.get_min(), 0.0)
  assert_eq(slider.get_max(), 100.0)
}

test "slider with value" {
  let slider = Slider::new("volume", 0.0, 0.0, 100.0).with_value(75.0)
  assert_eq(slider.get_value(), 75.0)
}

test "slider with step" {
  let slider = Slider::new("volume", 0.0, 0.0, 100.0).with_step(5.0)
  assert_eq(slider.get_step(), 5.0)
}

test "slider with disabled" {
  let slider = Slider::new("volume", 0.0, 0.0, 100.0).with_disabled(true)
  assert_eq(slider.is_slider_disabled(), true)
}

test "slider get progress" {
  let slider = Slider::new("volume", 50.0, 0.0, 100.0)
  assert_eq(slider.get_progress(), 0.5)
}

test "slider get progress quarter" {
  let slider = Slider::new("volume", 25.0, 0.0, 100.0)
  assert_eq(slider.get_progress(), 0.25)
}

test "slider get progress full" {
  let slider = Slider::new("volume", 100.0, 0.0, 100.0)
  assert_eq(slider.get_progress(), 1.0)
}

test "slider get progress zero" {
  let slider = Slider::new("volume", 0.0, 0.0, 100.0)
  assert_eq(slider.get_progress(), 0.0)
}

test "slider base widget" {
  let slider = Slider::new("volume", 0.0, 0.0, 100.0)
  let base = slider.base()
  assert_eq(base.get_type(), "slider")
}

test "slider default step" {
  let slider = Slider::new("volume", 0.0, 0.0, 100.0)
  assert_eq(slider.get_step(), 1.0)
}
