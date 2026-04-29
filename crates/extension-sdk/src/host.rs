use crate::bindings::vellum::extension;
use crate::decoration::{Decoration, Tooltip};
use crate::ui::UiNode;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct VersionedPayload<T> {
    pub version: u32,
    pub data: T,
}

impl<T> VersionedPayload<T> {
    pub fn new(data: T) -> Self {
        Self { version: 1, data }
    }
}

pub fn log(level: extension::types::LogLevel, message: &str) {
    extension::host::log(level, message);
}

pub fn show_status_message(message: &str) -> Result<(), String> {
    extension::host::show_status_message(message).map_err(|err| err.message)
}

pub fn document_text() -> Result<String, String> {
    extension::editor::document_text().map_err(|err| err.message)
}

pub fn document_path() -> Result<Option<String>, String> {
    extension::editor::document_path().map_err(|err| err.message)
}

pub fn replace_range(start: usize, end: usize, text: &str) -> Result<(), String> {
    extension::editor::replace_range(start as u64, end as u64, text).map_err(|err| err.message)
}

pub fn insert_text(position: usize, text: &str) -> Result<(), String> {
    extension::editor::insert_text(position as u64, text).map_err(|err| err.message)
}

pub fn set_panel_view(panel_id: &str, root: &UiNode) -> Result<(), String> {
    let bytes = serde_json::to_vec(&VersionedPayload::new(root))
        .map_err(|err| format!("failed to encode panel view: {err}"))?;
    extension::ui::set_panel_view(panel_id, &bytes).map_err(|err| err.message)
}

pub fn set_decorations(decorations: &[Decoration]) -> Result<(), String> {
    let bytes = serde_json::to_vec(&VersionedPayload::new(decorations))
        .map_err(|err| format!("failed to encode decorations: {err}"))?;
    extension::editor::set_decorations(&bytes).map_err(|err| err.message)
}

pub fn clear_decorations() -> Result<(), String> {
    extension::editor::clear_decorations().map_err(|err| err.message)
}

pub fn now_ms() -> u64 {
    extension::timer::now_ms()
}

pub fn request_tick(interval_ms: u32) -> Result<(), String> {
    extension::timer::request_tick(interval_ms).map_err(|err| err.message)
}

pub fn cancel_tick() -> Result<(), String> {
    extension::timer::cancel_tick().map_err(|err| err.message)
}

pub fn encode_tooltip(tooltip: Tooltip) -> Result<Vec<u8>, String> {
    serde_json::to_vec(&VersionedPayload::new(tooltip))
        .map_err(|err| format!("failed to encode tooltip: {err}"))
}
