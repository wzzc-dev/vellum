///|
/// ListView displays a list of items with a builder function.
pub struct ListView[T] {
  base : Widget
  items : @list.List[T]
  key_extractor : (T) -> String
}

///|
/// Create a new ListView with items and a builder function.
pub fn ListView::new[T](
  items : @list.List[T],
  key_extractor : (T) -> String,
) -> ListView[T] {
  {
    base: Widget::new("list-view", "list-view-gen-1"),
    items: items,
    key_extractor: key_extractor,
  }
}

///|
/// Create a ListView with default string keys.
pub fn ListView::with_strings(items : @list.List[String]) -> ListView[String] {
  ListView::new(items, fn(s) { s })
}

///|
/// Get the items in this ListView.
pub fn ListView::get_items[T](self : ListView[T]) -> @list.List[T] {
  self.items
}

///|
/// Get the number of items.
pub fn ListView::length[T](self : ListView[T]) -> Int {
  self.items.length()
}

///|
/// Check if the list is empty.
pub fn ListView::is_empty[T](self : ListView[T]) -> Bool {
  self.items.length() == 0
}

///|
/// Get the base widget.
pub fn ListView::base[T](self : ListView[T]) -> Widget {
  self.base
}
