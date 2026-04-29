#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ExtensionEvent {
    pub event_type: String,
    pub document_text: String,
    pub document_path: Option<String>,
}

impl ExtensionEvent {
    pub fn is_document_changed(&self) -> bool {
        self.event_type == "document.changed"
    }

    pub fn is_document_opened(&self) -> bool {
        self.event_type == "document.opened"
    }
}
