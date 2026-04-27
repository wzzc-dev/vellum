use serde::{Deserialize, Serialize};

/// Event data sent from host to extension.
/// Must match the SDK's EventData enum for postcard serialization compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventData {
    DocumentOpened { path: Option<String> },
    DocumentClosed { path: Option<String> },
    DocumentChanged { text: String, path: Option<String> },
    DocumentSaved { path: String },
    SelectionChanged { start: usize, end: usize },
    EditorFocused,
    EditorBlurred,
}
