use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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

pub fn decode_event(event_type: u32, data_ptr: u32, data_len: u32) -> EventData {
    let bytes = unsafe { core::slice::from_raw_parts(data_ptr as *const u8, data_len as usize) };
    let event_type = match event_type {
        0 => EventType::DocumentOpened,
        1 => EventType::DocumentClosed,
        2 => EventType::DocumentChanged,
        3 => EventType::DocumentSaved,
        4 => EventType::SelectionChanged,
        5 => EventType::EditorFocused,
        6 => EventType::EditorBlurred,
        _ => return EventData::EditorFocused,
    };
    match postcard::from_bytes(bytes) {
        Ok(data) => data,
        Err(_) => match event_type {
            EventType::EditorFocused => EventData::EditorFocused,
            EventType::EditorBlurred => EventData::EditorBlurred,
            _ => EventData::EditorFocused,
        },
    }
}
