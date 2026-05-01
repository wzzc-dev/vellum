///|
/// Dialog types.
pub enum DialogType {
  Alert
  Confirm
  Prompt
}

///|
/// Dialog represents a modal dialog.
pub struct Dialog {
  base : Widget
  title : String
  message : String
  is_open : Bool
  dialog_type : DialogType
  confirm_text : String
  cancel_text : String
}

///|
/// Create an alert dialog.
pub fn Dialog::alert(message : String) -> Dialog {
  {
    base: Widget::new("dialog", "dialog-gen-1"),
    title: "Alert",
    message: message,
    is_open: true,
    dialog_type: Alert,
    confirm_text: "OK",
    cancel_text: "",
  }
}

///|
/// Create a confirm dialog.
pub fn Dialog::confirm(message : String) -> Dialog {
  {
    base: Widget::new("dialog", "dialog-gen-1"),
    title: "Confirm",
    message: message,
    is_open: true,
    dialog_type: Confirm,
    confirm_text: "OK",
    cancel_text: "Cancel",
  }
}

///|
/// Create a prompt dialog.
pub fn Dialog::prompt(message : String) -> Dialog {
  {
    base: Widget::new("dialog", "dialog-gen-1"),
    title: "Prompt",
    message: message,
    is_open: true,
    dialog_type: Prompt,
    confirm_text: "Submit",
    cancel_text: "Cancel",
  }
}

///|
/// Close the dialog.
pub fn Dialog::close(self : Dialog) -> Dialog {
  { ..self, is_open: false }
}

///|
/// Open the dialog.
pub fn Dialog::open(self : Dialog) -> Dialog {
  { ..self, is_open: true }
}

///|
/// Set the title.
pub fn Dialog::with_title(self : Dialog, title : String) -> Dialog {
  { ..self, title: title }
}

///|
/// Get the dialog title.
pub fn Dialog::get_title(self : Dialog) -> String {
  self.title
}

///|
/// Get the dialog message.
pub fn Dialog::get_message(self : Dialog) -> String {
  self.message
}

///|
/// Check if the dialog is open.
pub fn Dialog::is_open(self : Dialog) -> Bool {
  self.is_open
}

///|
/// Get the dialog type.
pub fn Dialog::get_type(self : Dialog) -> DialogType {
  self.dialog_type
}

///|
/// Set the confirm button text.
pub fn Dialog::with_confirm_text(self : Dialog, text : String) -> Dialog {
  { ..self, confirm_text: text }
}

///|
/// Set the cancel button text.
pub fn Dialog::with_cancel_text(self : Dialog, text : String) -> Dialog {
  { ..self, cancel_text: text }
}

///|
/// Get the base widget.
pub fn Dialog::base(self : Dialog) -> Widget {
  self.base
}
