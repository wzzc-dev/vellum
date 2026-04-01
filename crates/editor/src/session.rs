use gpui::Entity;
use gpui_component::input::InputState;

#[derive(Debug, Clone)]
pub struct ActiveBlockSession {
    pub block_id: u64,
    pub input: Entity<InputState>,
}

impl ActiveBlockSession {
    pub fn new(block_id: u64, input: Entity<InputState>) -> Self {
        Self { block_id, input }
    }
}
