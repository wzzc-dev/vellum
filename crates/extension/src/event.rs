#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionEvent {
    pub event_type: String,
    pub document_text: String,
    pub document_path: Option<String>,
    pub timestamp_ms: Option<u64>,
}

impl ExtensionEvent {
    pub fn document_changed(text: String, path: Option<String>) -> Self {
        Self {
            event_type: "document.changed".into(),
            document_text: text,
            document_path: path,
            timestamp_ms: None,
        }
    }

    pub fn document_opened(text: String, path: Option<String>) -> Self {
        Self {
            event_type: "document.opened".into(),
            document_text: text,
            document_path: path,
            timestamp_ms: None,
        }
    }
}
