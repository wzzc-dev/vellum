///|
/// Tests for SelectOption.
test "select option new" {
  let option = SelectOption::new("red", "Red")
  assert_eq(option.get_value(), "red")
  assert_eq(option.get_label(), "Red")
}

///|
/// Tests for Select.
test "select new" {
  let options = @list.empty()
    .add(SelectOption::new("red", "Red"))
    .add(SelectOption::new("green", "Green"))
  let select = Select::new("color", options)
  assert_eq(select.option_count(), 2)
  assert_eq(select.get_selected(), "")
}

test "select with selected" {
  let options = @list.empty()
    .add(SelectOption::new("red", "Red"))
    .add(SelectOption::new("green", "Green"))
  let select = Select::new("color", options).with_selected("red")
  assert_eq(select.get_selected(), "red")
}

test "select with disabled" {
  let options = @list.empty()
    .add(SelectOption::new("red", "Red"))
  let select = Select::new("color", options).with_disabled(true)
  assert_eq(select.is_select_disabled(), true)
}

test "select with open" {
  let options = @list.empty()
    .add(SelectOption::new("red", "Red"))
  let select = Select::new("color", options).with_open(true)
  assert_eq(select.is_dropdown_open(), true)
}

test "select toggle dropdown" {
  let options = @list.empty()
    .add(SelectOption::new("red", "Red"))
  let select = Select::new("color", options).toggle_dropdown()
  assert_eq(select.is_dropdown_open(), true)
}

test "select toggle dropdown twice" {
  let options = @list.empty()
    .add(SelectOption::new("red", "Red"))
  let select = Select::new("color", options).toggle_dropdown().toggle_dropdown()
  assert_eq(select.is_dropdown_open(), false)
}

test "select base widget" {
  let options = @list.empty()
  let select = Select::new("color", options)
  let base = select.base()
  assert_eq(base.get_type(), "select")
}

test "select empty options" {
  let select = Select::new("empty", @list.empty())
  assert_eq(select.option_count(), 0)
}
