use crate::decoration::{Decoration, DecorationKind, UnderlineStyle};
use crate::event::ExtensionEvent;
use crate::host::VersionedPayload;
use crate::manifest::ExtensionManifest;
use crate::ui::{ButtonVariant, Severity, TextStyle, UiNode};

#[test]
fn manifest_serializes_as_json() {
    let manifest = ExtensionManifest {
        id: "test.extension".into(),
        name: "Test".into(),
        version: "0.1.0".into(),
        description: "Test extension".into(),
        authors: vec!["Vellum".into()],
    };

    let bytes = serde_json::to_vec(&manifest).unwrap();
    let decoded: ExtensionManifest = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(manifest.id, decoded.id);
    assert_eq!(manifest.authors, decoded.authors);
}

#[test]
fn event_helpers_match_zed_style_activation_events() {
    let opened = ExtensionEvent {
        event_type: "document.opened".into(),
        document_text: "# Title".into(),
        document_path: Some("/test.md".into()),
    };
    let changed = ExtensionEvent {
        event_type: "document.changed".into(),
        document_text: "# Changed".into(),
        document_path: None,
    };

    assert!(opened.is_document_opened());
    assert!(!opened.is_document_changed());
    assert!(changed.is_document_changed());
}

#[test]
fn ui_node_payload_uses_versioned_json() {
    let node = UiNode::column()
        .gap(8.0)
        .padding(12.0)
        .child(UiNode::heading("Title", 2))
        .child(
            UiNode::row()
                .gap(4.0)
                .child(UiNode::badge("MD001").severity(Severity::Error).build())
                .child(UiNode::link("link1", "Click here"))
                .build(),
        )
        .child(UiNode::separator())
        .child(UiNode::styled_text(
            "muted text",
            TextStyle::small().muted(),
        ))
        .child(
            UiNode::button("btn1", "Run")
                .variant(ButtonVariant::Primary)
                .icon("play")
                .build(),
        )
        .child(
            UiNode::disclosure("Details")
                .open(true)
                .child(UiNode::text("Content"))
                .build(),
        )
        .build();

    let bytes = serde_json::to_vec(&VersionedPayload::new(node)).unwrap();
    let decoded: VersionedPayload<UiNode> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(decoded.version, 1);
    assert!(matches!(decoded.data, UiNode::Column { .. }));
}

#[test]
fn text_style_builder_sets_expected_flags() {
    let style = TextStyle::small().muted().bold().italic().monospace();
    assert_eq!(style.size, Some(11.0));
    assert_eq!(style.color.as_deref(), Some("muted-foreground"));
    assert_eq!(style.bold, Some(true));
    assert_eq!(style.italic, Some(true));
    assert_eq!(style.monospace, Some(true));
}

#[test]
fn decoration_payload_uses_versioned_json() {
    let decorations = vec![Decoration {
        id: "d1".into(),
        start: 5,
        end: 10,
        kind: DecorationKind::Underline {
            color: "red".into(),
            style: UnderlineStyle::Wavy,
        },
        tooltip: Some("error".into()),
        hover_data: None,
    }];

    let bytes = serde_json::to_vec(&VersionedPayload::new(decorations)).unwrap();
    let decoded: VersionedPayload<Vec<Decoration>> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(decoded.version, 1);
    assert_eq!(decoded.data.len(), 1);
}
