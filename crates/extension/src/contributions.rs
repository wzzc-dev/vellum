use serde::{Deserialize, Serialize};

/// Serialize event data for passing to plugin_handle_event.
pub fn serialize_event_data(
    event_type: &str,
    document_id: &str,
    document_text: &str,
    document_path: Option<&str>,
) -> Vec<u8> {
    #[derive(Serialize)]
    struct EventPayload<'a> {
        event_type: &'a str,
        document_id: &'a str,
        document_text: &'a str,
        document_path: Option<&'a str>,
    }
    let payload = EventPayload {
        event_type,
        document_id,
        document_text,
        document_path,
    };
    postcard::to_allocvec(&payload).unwrap_or_default()
}

/// A registered command from an extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredCommand {
    pub id: u32,
    pub command_id: String,
    pub label: String,
    pub key_binding: Option<String>,
    pub extension_id: String,
}

/// A registered sidebar panel from an extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredPanel {
    pub id: u32,
    pub panel_id: String,
    pub label: String,
    pub icon: String,
    pub extension_id: String,
}

/// A pending edit operation from an extension.
#[derive(Debug, Clone)]
pub enum PendingEdit {
    Insert(String),
    ReplaceRange {
        start: usize,
        end: usize,
        text: String,
    },
}

/// A webview request from the host to an extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebViewRequest {
    pub webview_id: String,
    pub url: String,
    pub method: String,
    pub headers: Vec<(String, String)>,
}

/// A protocol response from an extension for a webview request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolResponse {
    pub mime_type: String,
    pub body: Vec<u8>,
}

/// A decoration applied to a document range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decoration {
    pub id: String,
    pub start: usize,
    pub end: usize,
    pub kind: DecorationKind,
    pub tooltip: Option<String>,
    pub hover_data: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DecorationKind {
    Underline {
        color: String,
        style: UnderlineStyle,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnderlineStyle {
    Solid,
    Dashed,
    Dotted,
    Wavy,
}

/// A tooltip shown by an extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tooltip {
    pub content: crate::ui::UiNode,
    pub position: TooltipPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TooltipPosition {
    Above,
    Below,
    Left,
    Right,
}

/// An overlay panel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayPanel {
    pub id: String,
    pub title: String,
    pub content: crate::ui::UiNode,
}
