use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u32)]
pub enum EventType {
    DocumentOpened = 0,
    DocumentClosed = 1,
    DocumentChanged = 2,
    DocumentSaved = 3,
    SelectionChanged = 4,
    EditorFocused = 5,
    EditorBlurred = 6,
}

impl TryFrom<u32> for EventType {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::DocumentOpened),
            1 => Ok(Self::DocumentClosed),
            2 => Ok(Self::DocumentChanged),
            3 => Ok(Self::DocumentSaved),
            4 => Ok(Self::SelectionChanged),
            5 => Ok(Self::EditorFocused),
            6 => Ok(Self::EditorBlurred),
            _ => anyhow::bail!("unknown event type: {}", value),
        }
    }
}

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

impl EventData {
    pub fn event_type(&self) -> EventType {
        match self {
            Self::DocumentOpened { .. } => EventType::DocumentOpened,
            Self::DocumentClosed { .. } => EventType::DocumentClosed,
            Self::DocumentChanged { .. } => EventType::DocumentChanged,
            Self::DocumentSaved { .. } => EventType::DocumentSaved,
            Self::SelectionChanged { .. } => EventType::SelectionChanged,
            Self::EditorFocused => EventType::EditorFocused,
            Self::EditorBlurred => EventType::EditorBlurred,
        }
    }
}
