use gpui::Entity;
use gpui_component::input::InputState;

use crate::{BlockSpan, DocumentState};

#[derive(Debug, Clone)]
pub struct ActiveBlockSession {
    pub block_id: u64,
    pub buffer: String,
    pub cursor_offset: usize,
    pub anchor_document_offset: usize,
    pub input: Entity<InputState>,
}

impl ActiveBlockSession {
    pub fn new(document: &DocumentState, block: &BlockSpan, input: Entity<InputState>) -> Self {
        Self {
            block_id: block.id,
            buffer: document.block_text(block),
            cursor_offset: 0,
            anchor_document_offset: block.byte_range.start,
            input,
        }
    }
}
