///|
/// Tests for ProgressBar.
test "progress bar new" {
  let progress = ProgressBar::new(50.0)
  assert_eq(progress.get_value(), 50.0)
  assert_eq(progress.get_max(), 100.0)
}

test "progress bar with value" {
  let progress = ProgressBar::new(0.0).with_value(75.0)
  assert_eq(progress.get_value(), 75.0)
}

test "progress bar with max" {
  let progress = ProgressBar::new(50.0).with_max(200.0)
  assert_eq(progress.get_max(), 200.0)
}

test "progress bar with indeterminate" {
  let progress = ProgressBar::new(0.0).with_indeterminate(true)
  assert_eq(progress.is_indeterminate_progress(), true)
}

test "progress bar get progress" {
  let progress = ProgressBar::new(50.0)
  assert_eq(progress.get_progress(), 0.5)
}

test "progress bar get progress quarter" {
  let progress = ProgressBar::new(25.0)
  assert_eq(progress.get_progress(), 0.25)
}

test "progress bar get progress full" {
  let progress = ProgressBar::new(100.0)
  assert_eq(progress.get_progress(), 1.0)
}

test "progress bar get progress zero" {
  let progress = ProgressBar::new(0.0)
  assert_eq(progress.get_progress(), 0.0)
}

test "progress bar base widget" {
  let progress = ProgressBar::new(50.0)
  let base = progress.base()
  assert_eq(base.get_type(), "progress-bar")
}

test "progress bar default not indeterminate" {
  let progress = ProgressBar::new(50.0)
  assert_eq(progress.is_indeterminate_progress(), false)
}
