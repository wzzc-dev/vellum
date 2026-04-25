use std::collections::HashMap;

use crate::decoration::{Decoration, OverlayPanel, RegisteredPanel, Tooltip};
use crate::ui::UiNode;

pub enum PendingEdit {
    Insert(String),
    ReplaceRange {
        start: usize,
        end: usize,
        text: String,
    },
}

pub struct HostState {
    pub plugin_id: String,
    pub alloc_offset: u32,
    pub next_command_id: u32,
    pub next_panel_id: u32,
    pub next_subscription_id: u32,
    pub pending_commands: Vec<crate::command::RegisteredCommand>,
    pub pending_panels: Vec<RegisteredPanel>,
    pub pending_subscriptions: Vec<(u32, u32)>,
    pub status_message: Option<String>,
    pub document_text: String,
    pub document_path: Option<String>,
    pub panel_uis: HashMap<u32, UiNode>,
    pub decorations: Vec<Decoration>,
    pub active_overlay: Option<OverlayPanel>,
    pub active_tooltip: Option<Tooltip>,
    pub pending_edits: Vec<PendingEdit>,
}

impl HostState {
    pub fn new(plugin_id: String) -> Self {
        Self {
            plugin_id,
            alloc_offset: 65536,
            next_command_id: 1,
            next_panel_id: 1,
            next_subscription_id: 1,
            pending_commands: Vec::new(),
            pending_panels: Vec::new(),
            pending_subscriptions: Vec::new(),
            status_message: None,
            document_text: String::new(),
            document_path: None,
            panel_uis: HashMap::new(),
            decorations: Vec::new(),
            active_overlay: None,
            active_tooltip: None,
            pending_edits: Vec::new(),
        }
    }

    pub fn update_document(&mut self, text: String, path: Option<String>) {
        self.document_text = text;
        self.document_path = path;
    }

    pub fn take_commands(&mut self) -> Vec<crate::command::RegisteredCommand> {
        std::mem::take(&mut self.pending_commands)
    }

    pub fn take_panels(&mut self) -> Vec<RegisteredPanel> {
        std::mem::take(&mut self.pending_panels)
    }

    pub fn take_subscriptions(&mut self) -> Vec<(u32, u32)> {
        std::mem::take(&mut self.pending_subscriptions)
    }

    pub fn take_status_message(&mut self) -> Option<String> {
        std::mem::take(&mut self.status_message)
    }

    pub fn take_edits(&mut self) -> Vec<PendingEdit> {
        std::mem::take(&mut self.pending_edits)
    }
}
