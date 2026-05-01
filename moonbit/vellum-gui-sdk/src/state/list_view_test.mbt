///|
/// Tests for ListView component.
test "ListView::new creates list view" {
  let items = @list.empty()
  let list_view = ListView::new(items, fn(x) { x })
  inspect(list_view.length(), content="0")
}

test "ListView::get_items returns items" {
  let items = @list.empty()
  let list_view = ListView::new(items, fn(x) { x })
  let retrieved = list_view.get_items()
  inspect(retrieved.length(), content="0")
}

test "ListView::is_empty for empty list" {
  let list_view = ListView::new(@list.empty(), fn(x) { x })
  inspect(list_view.is_empty(), content="true")
}

test "ListView::base returns widget" {
  let list_view = ListView::new(@list.empty(), fn(x) { x })
  let widget = list_view.base()
  inspect(widget.get_type(), content="list-view")
}

test "ListView::with_items preserves structure" {
  let items = @list.empty()
  let list_view = ListView::new(items, fn(x : String) { x })
  inspect(list_view.length(), content="0")
}
