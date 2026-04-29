#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ExtensionEvent {
    pub event_type: String,
    pub document_text: String,
    pub document_path: Option<String>,
    pub timestamp_ms: Option<u64>,
}

impl ExtensionEvent {
    pub fn is_document_changed(&self) -> bool {
        self.event_type == "document.changed"
    }

    pub fn is_document_opened(&self) -> bool {
        self.event_type == "document.opened"
    }

    pub fn is_timer_tick(&self) -> bool {
        self.event_type == "timer.tick"
    }
}
