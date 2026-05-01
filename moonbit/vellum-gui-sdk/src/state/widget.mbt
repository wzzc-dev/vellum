///|
/// Widget represents the base type for all UI components.
pub struct Widget {
  id : String
  widget_type : String
  children : @list.List[Widget]
  properties : @list.List[(String, String)]
}

///|
/// Create a new Widget with the given type and ID.
pub fn Widget::new(widget_type : String, id : String) -> Widget {
  {
    id: id,
    widget_type: widget_type,
    children: @list.empty(),
    properties: @list.empty(),
  }
}

///|
/// Get the ID of this widget.
pub fn Widget::get_id(self : Widget) -> String {
  self.id
}

///|
/// Get the type of this widget.
pub fn Widget::get_type(self : Widget) -> String {
  self.widget_type
}

///|
/// Add a child widget.
pub fn Widget::add_child(self : Widget, child : Widget) -> Widget {
  { ..self, children: self.children.add(child) }
}

///|
/// Set a property on this widget.
pub fn Widget::set_property(self : Widget, key : String, value : String) -> Widget {
  { ..self, properties: self.properties.add((key, value)) }
}

///|
/// Get the number of children.
pub fn Widget::child_count(self : Widget) -> Int {
  self.children.length()
}
