///|
/// Comprehensive tests for Container.
test "column creation" {
  let col = Container::column(@list.empty())
  match col.get_type() {
    Column => ()
    _ => assert_eq(0, 1)
  }
  assert_eq(col.get_spacing(), 0.0)
  assert_eq(col.get_padding(), 0.0)
}

test "row creation" {
  let row = Container::row(@list.empty())
  match row.get_type() {
    Row => ()
    _ => assert_eq(0, 1)
  }
}

test "stack creation" {
  let stack = Container::stack(@list.empty())
  match stack.get_type() {
    Stack => ()
    _ => assert_eq(0, 1)
  }
}

test "flex creation" {
  let flex = Container::flex(@list.empty())
  match flex.get_type() {
    Flex => ()
    _ => assert_eq(0, 1)
  }
}

test "column with children" {
  let child1 = Widget::new("text", "txt-1")
  let child2 = Widget::new("text", "txt-2")
  let children = @list.empty().add(child1).add(child2)
  let col = Container::column(children)
  assert_eq(col.base().child_count(), 2)
}

test "column with spacing" {
  let col = Container::column(@list.empty()).with_spacing(16.0)
  assert_eq(col.get_spacing(), 16.0)
}

test "column with padding" {
  let col = Container::column(@list.empty()).with_padding(24.0)
  assert_eq(col.get_padding(), 24.0)
}

test "column chain spacing and padding" {
  let col = Container::column(@list.empty())
    .with_spacing(10.0)
    .with_padding(20.0)
  assert_eq(col.get_spacing(), 10.0)
  assert_eq(col.get_padding(), 20.0)
}

test "row with children and properties" {
  let child = Widget::new("button", "btn-1")
  let row = Container::row(@list.empty().add(child))
    .with_spacing(8.0)
    .with_padding(16.0)
  assert_eq(row.base().child_count(), 1)
  assert_eq(row.get_spacing(), 8.0)
  assert_eq(row.get_padding(), 16.0)
}

test "stack with multiple children" {
  let bg = Widget::new("rect", "bg")
  let fg = Widget::new("text", "fg")
  let stack = Container::stack(@list.empty().add(bg).add(fg))
  assert_eq(stack.base().child_count(), 2)
  match stack.get_type() {
    Stack => ()
    _ => assert_eq(0, 1)
  }
}
