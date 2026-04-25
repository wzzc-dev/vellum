use crate::decoration::{Decoration, OverlayPanel, ProtocolResponse, Tooltip, WebViewRequest};
use crate::event::EventData;
use crate::manifest::PluginManifest;
use crate::ui::UiEvent;

pub fn encode_manifest(manifest: &PluginManifest) -> Vec<u8> {
    postcard::to_allocvec(manifest).expect("failed to serialize manifest")
}

pub fn decode_manifest(bytes: &[u8]) -> anyhow::Result<PluginManifest> {
    Ok(postcard::from_bytes(bytes)?)
}

pub fn encode_event_data(event: &EventData) -> Vec<u8> {
    postcard::to_allocvec(event).expect("failed to serialize event data")
}

pub fn decode_event_data(bytes: &[u8]) -> anyhow::Result<EventData> {
    Ok(postcard::from_bytes(bytes)?)
}

pub fn encode_ui_event(event: &UiEvent) -> Vec<u8> {
    postcard::to_allocvec(event).expect("failed to serialize ui event")
}

pub fn decode_ui_event(bytes: &[u8]) -> anyhow::Result<UiEvent> {
    Ok(postcard::from_bytes(bytes)?)
}

pub fn encode_decorations(decorations: &[Decoration]) -> Vec<u8> {
    postcard::to_allocvec(decorations).expect("failed to serialize decorations")
}

pub fn decode_decorations(bytes: &[u8]) -> anyhow::Result<Vec<Decoration>> {
    Ok(postcard::from_bytes(bytes)?)
}

pub fn encode_tooltip(tooltip: &Option<Tooltip>) -> Vec<u8> {
    postcard::to_allocvec(tooltip).expect("failed to serialize tooltip")
}

pub fn decode_tooltip(bytes: &[u8]) -> anyhow::Result<Option<Tooltip>> {
    Ok(postcard::from_bytes(bytes)?)
}

pub fn encode_overlay(overlay: &OverlayPanel) -> Vec<u8> {
    postcard::to_allocvec(overlay).expect("failed to serialize overlay")
}

pub fn decode_overlay(bytes: &[u8]) -> anyhow::Result<OverlayPanel> {
    Ok(postcard::from_bytes(bytes)?)
}

pub fn encode_webview_request(request: &WebViewRequest) -> Vec<u8> {
    postcard::to_allocvec(request).expect("failed to serialize webview request")
}

pub fn decode_webview_request(bytes: &[u8]) -> anyhow::Result<WebViewRequest> {
    Ok(postcard::from_bytes(bytes)?)
}

pub fn encode_protocol_response(response: &ProtocolResponse) -> Vec<u8> {
    postcard::to_allocvec(response).expect("failed to serialize protocol response")
}

pub fn decode_protocol_response(bytes: &[u8]) -> anyhow::Result<ProtocolResponse> {
    Ok(postcard::from_bytes(bytes)?)
}
