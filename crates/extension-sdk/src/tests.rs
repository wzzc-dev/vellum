use crate::decoration::{Decoration, DecorationKind, UnderlineStyle};
use crate::event::{EventData, EventType};
use crate::manifest::PluginManifest;
use crate::ui::{ButtonVariant, Severity, TextStyle, UiNode};

#[test]
fn test_manifest_serialization() {
    let manifest = PluginManifest {
        id: "test.plugin".into(),
        name: "Test".into(),
        version: "0.1.0".into(),
        description: "Test plugin".into(),
        author: "Author".into(),
    };
    let bytes = postcard::to_allocvec(&manifest).unwrap();
    let decoded: PluginManifest = postcard::from_bytes(&bytes).unwrap();
    assert_eq!(manifest.id, decoded.id);
}

#[test]
fn test_event_type_values() {
    assert_eq!(EventType::DocumentOpened as u32, 0);
    assert_eq!(EventType::DocumentClosed as u32, 1);
    assert_eq!(EventType::DocumentChanged as u32, 2);
    assert_eq!(EventType::DocumentSaved as u32, 3);
    assert_eq!(EventType::SelectionChanged as u32, 4);
    assert_eq!(EventType::EditorFocused as u32, 5);
    assert_eq!(EventType::EditorBlurred as u32, 6);
}

#[test]
fn test_event_data_serialization() {
    let event = EventData::DocumentChanged {
        text: "hello".into(),
        path: Some("/test.md".into()),
    };
    let bytes = postcard::to_allocvec(&event).unwrap();
    let decoded: EventData = postcard::from_bytes(&bytes).unwrap();
    assert!(matches!(decoded, EventData::DocumentChanged { .. }));
}

#[test]
fn test_ui_node_builder() {
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
        .child(UiNode::styled_text("muted text", TextStyle::small().muted()))
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

    let bytes = postcard::to_allocvec(&node).unwrap();
    let decoded: UiNode = postcard::from_bytes(&bytes).unwrap();
    assert!(matches!(decoded, UiNode::Column { .. }));
}

#[test]
fn test_text_style_builder() {
    let style = TextStyle::small().muted().bold().italic().monospace();
    assert_eq!(style.size, Some(11.0));
    assert_eq!(style.color.as_deref(), Some("muted-foreground"));
    assert_eq!(style.bold, Some(true));
    assert_eq!(style.italic, Some(true));
    assert_eq!(style.monospace, Some(true));
}

#[test]
fn test_decoration_serialization() {
    let decos = vec![
        Decoration {
            id: "d1".into(),
            start: 5,
            end: 10,
            kind: DecorationKind::Underline {
                color: "red".into(),
                style: UnderlineStyle::Wavy,
            },
            tooltip: Some("error".into()),
            hover_data: None,
        },
    ];
    let bytes = postcard::to_allocvec(&decos).unwrap();
    let decoded: Vec<Decoration> = postcard::from_bytes(&bytes).unwrap();
    assert_eq!(decos.len(), decoded.len());
}
