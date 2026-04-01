use super::component_ui::BlockInput;

#[derive(Debug, Clone)]
pub(crate) struct ActiveBlockSession {
    pub(crate) block_id: u64,
    pub(crate) input: BlockInput,
}

impl ActiveBlockSession {
    pub(crate) fn new(block_id: u64, input: BlockInput) -> Self {
        Self { block_id, input }
    }
}
