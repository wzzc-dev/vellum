///|
/// Tests for Dialog component.
test "Dialog::alert creates dialog" {
  let dialog = Dialog::alert("Hello!")
  inspect(dialog.get_title(), content="Alert")
  inspect(dialog.get_message(), content="Hello!")
  inspect(dialog.is_open(), content="true")
}

test "Dialog::confirm creates dialog" {
  let dialog = Dialog::confirm("Are you sure?")
  inspect(dialog.get_title(), content="Confirm")
  inspect(dialog.is_open(), content="true")
}

test "Dialog::prompt creates dialog" {
  let dialog = Dialog::prompt("Enter name:")
  inspect(dialog.get_title(), content="Prompt")
}

test "Dialog::close closes dialog" {
  let dialog = Dialog::alert("Test")
  let closed = dialog.close()
  inspect(closed.is_open(), content="false")
}

test "Dialog::open opens dialog" {
  let dialog = Dialog::alert("Test").close()
  let opened = dialog.open()
  inspect(opened.is_open(), content="true")
}

test "Dialog::with_title changes title" {
  let dialog = Dialog::alert("Test").with_title("Custom Title")
  inspect(dialog.get_title(), content="Custom Title")
}

test "Dialog::with_confirm_text changes button text" {
  let dialog = Dialog::confirm("Test").with_confirm_text("Yes")
  inspect(dialog.confirm_text, content="Yes")
}

test "Dialog::with_cancel_text changes cancel text" {
  let dialog = Dialog::confirm("Test").with_cancel_text("No")
  inspect(dialog.cancel_text, content="No")
}

test "Dialog::base returns widget" {
  let dialog = Dialog::alert("Test")
  let widget = dialog.base()
  inspect(widget.get_type(), content="dialog")
}

test "Dialog::get_message returns message" {
  let dialog = Dialog::alert("Hello World")
  inspect(dialog.get_message(), content="Hello World")
}

test "Dialog is_open starts true" {
  let dialog = Dialog::alert("Test")
  inspect(dialog.is_open(), content="true")
}
