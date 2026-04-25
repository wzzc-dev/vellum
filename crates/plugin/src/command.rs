use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredCommand {
    pub id: u32,
    pub command_id: String,
    pub label: String,
    pub key_binding: Option<String>,
    pub plugin_id: String,
}
