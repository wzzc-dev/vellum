use crate::event::{EventData, EventType};
use crate::manifest::PluginManifest;
use crate::protocol;

#[test]
fn test_manifest_roundtrip() {
    let manifest = PluginManifest {
        id: "test.plugin".into(),
        name: "Test Plugin".into(),
        version: "1.0.0".into(),
        description: "A test plugin".into(),
        author: "Test Author".into(),
    };
    let encoded = protocol::encode_manifest(&manifest);
    let decoded = protocol::decode_manifest(&encoded).unwrap();
    assert_eq!(manifest.id, decoded.id);
    assert_eq!(manifest.name, decoded.name);
    assert_eq!(manifest.version, decoded.version);
    assert_eq!(manifest.description, decoded.description);
    assert_eq!(manifest.author, decoded.author);
}

#[test]
fn test_event_data_roundtrip() {
    let events = vec![
        EventData::DocumentOpened { path: Some("/test.md".into()) },
        EventData::DocumentClosed { path: None },
        EventData::DocumentChanged { text: "hello world".into(), path: Some("/test.md".into()) },
        EventData::DocumentSaved { path: "/test.md".into() },
        EventData::SelectionChanged { start: 5, end: 10 },
        EventData::EditorFocused,
        EventData::EditorBlurred,
    ];

    for event in events {
        let encoded = protocol::encode_event_data(&event);
        let decoded = protocol::decode_event_data(&encoded).unwrap();
        assert_eq!(event.event_type(), decoded.event_type());
    }
}

#[test]
fn test_event_type_try_from() {
    assert_eq!(EventType::try_from(0).unwrap(), EventType::DocumentOpened);
    assert_eq!(EventType::try_from(1).unwrap(), EventType::DocumentClosed);
    assert_eq!(EventType::try_from(2).unwrap(), EventType::DocumentChanged);
    assert_eq!(EventType::try_from(3).unwrap(), EventType::DocumentSaved);
    assert_eq!(EventType::try_from(4).unwrap(), EventType::SelectionChanged);
    assert_eq!(EventType::try_from(5).unwrap(), EventType::EditorFocused);
    assert_eq!(EventType::try_from(6).unwrap(), EventType::EditorBlurred);
    assert!(EventType::try_from(7).is_err());
}

#[test]
fn test_ui_node_roundtrip() {
    use crate::ui::{ButtonVariant, EdgeInsets, Severity, TextStyle, UiNode};

    let node = UiNode::Column {
        children: vec![
            UiNode::Heading {
                content: "Test".into(),
                level: 2,
            },
            UiNode::Row {
                children: vec![
                    UiNode::Button {
                        id: "btn1".into(),
                        label: "Click".into(),
                        variant: ButtonVariant::Primary,
                        icon: Some("play".into()),
                        disabled: false,
                    },
                    UiNode::Badge {
                        label: "MD001".into(),
                        severity: Some(Severity::Error),
                    },
                ],
                gap: Some(8.0),
                padding: Some(EdgeInsets::uniform(4.0)),
            },
            UiNode::Text {
                content: "Hello".into(),
                style: TextStyle::small().muted().bold(),
            },
            UiNode::Separator,
            UiNode::Spacer,
        ],
        gap: Some(12.0),
        padding: None,
        scrollable: true,
    };

    let encoded = postcard::to_allocvec(&node).unwrap();
    let decoded: UiNode = postcard::from_bytes(&encoded).unwrap();

    if let UiNode::Column { children, gap, scrollable, .. } = decoded {
        assert_eq!(children.len(), 5);
        assert_eq!(gap, Some(12.0));
        assert!(scrollable);
    } else {
        panic!("expected Column node");
    }
}

#[test]
fn test_decoration_roundtrip() {
    use crate::decoration::{Decoration, DecorationKind, UnderlineStyle};

    let decorations = vec![
        Decoration {
            id: "lint-0".into(),
            start: 10,
            end: 20,
            kind: DecorationKind::Underline {
                color: "red".into(),
                style: UnderlineStyle::Wavy,
            },
            tooltip: Some("MD022: Error".into()),
            hover_data: Some("0".into()),
        },
        Decoration {
            id: "lint-1".into(),
            start: 30,
            end: 40,
            kind: DecorationKind::Highlight {
                color: "yellow".into(),
            },
            tooltip: None,
            hover_data: None,
        },
    ];

    let encoded = protocol::encode_decorations(&decorations);
    let decoded = protocol::decode_decorations(&encoded).unwrap();
    assert_eq!(decorations.len(), decoded.len());
    assert_eq!(decorations[0].id, decoded[0].id);
    assert_eq!(decorations[1].id, decoded[1].id);
}

#[test]
fn test_ui_event_roundtrip() {
    use crate::ui::UiEvent;

    let events = vec![
        UiEvent::ButtonClicked { element_id: "btn1".into() },
        UiEvent::InputChanged { element_id: "input1".into(), value: "hello".into() },
        UiEvent::CheckboxToggled { element_id: "chk1".into(), checked: true },
        UiEvent::SelectChanged { element_id: "sel1".into(), index: 2 },
        UiEvent::LinkClicked { element_id: "link1".into() },
    ];

    for event in &events {
        let encoded = protocol::encode_ui_event(event);
        let decoded = protocol::decode_ui_event(&encoded).unwrap();
        assert_eq!(std::mem::discriminant(event), std::mem::discriminant(&decoded));
    }
}

#[test]
fn test_plugin_manager_new() {
    let manager = crate::manager::PluginManager::new();
    assert!(manager.is_ok());
    let manager = manager.unwrap();
    assert!(manager.commands().is_empty());
    assert!(manager.sidebar_panels().is_empty());
    assert!(manager.decorations().is_empty());
}

#[test]
fn test_plugin_manager_update_document() {
    let mut manager = crate::manager::PluginManager::new().unwrap();
    manager.update_document("hello world".into(), Some("/test.md".into()));
}
