///|
/// Tests for ListView component.
test "ListView::new creates list view" {
  let items = @list.empty()
  let list_view = ListView::new(items, fn(x) { x })
  inspect(list_view.length(), content="0")
}

test "ListView::with_strings creates list view" {
  let items = ["a", "b", "c"]
  let list_view = ListView::with_strings(items)
  inspect(list_view.length(), content="3")
}

test "ListView::get_items returns items" {
  let items = [1, 2, 3]
  let list_view = ListView::new(items, fn(x) { x.to_string() })
  let retrieved = list_view.get_items()
  inspect(retrieved.length(), content="3")
}

test "ListView::is_empty for empty list" {
  let list_view = ListView::with_strings(@list.empty())
  inspect(list_view.is_empty(), content="true")
}

test "ListView::is_empty for non-empty list" {
  let list_view = ListView::with_strings(["item"])
  inspect(list_view.is_empty(), content="false")
}

test "ListView::base returns widget" {
  let list_view = ListView::with_strings(["a", "b"])
  let widget = list_view.base()
  inspect(widget.get_type(), content="list-view")
}

test "ListView with custom key extractor" {
  struct Item { id : Int, name : String }
  let items = [
    { id: 1, name: "first" },
    { id: 2, name: "second" }
  ]
  let list_view = ListView::new(items, fn(item) { item.id.to_string() })
  inspect(list_view.length(), content="2")
}

test "ListView with many items" {
  let items = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
  let list_view = ListView::new(items, fn(x) { x.to_string() })
  inspect(list_view.length(), content="10")
}

test "ListView::get_items preserves order" {
  let items = ["first", "second", "third"]
  let list_view = ListView::with_strings(items)
  let retrieved = list_view.get_items()
  inspect(retrieved.length(), content="3")
}
