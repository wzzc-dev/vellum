use thiserror::Error;

#[derive(Error, Debug)]
pub enum AdapterError {
    #[error("widget not found: {0}")]
    WidgetNotFound(String),

    #[error("window not found: {0}")]
    WindowNotFound(u32),

    #[error("invalid widget type: {0}")]
    InvalidWidgetType(String),

    #[error("mount failed: {0}")]
    MountFailed(String),

    #[error("render failed: {0}")]
    RenderFailed(String),

    #[error("event dispatch failed: {0}")]
    EventDispatchFailed(String),

    #[error("serialization error: {0}")]
    SerializationError(String),

    #[error("io error: {0}")]
    IoError(String),

    #[error("invalid argument: {0}")]
    InvalidArgument(String),
}

pub type Result<T> = std::result::Result<T, AdapterError>;

impl From<serde_json::Error> for AdapterError {
    fn from(err: serde_json::Error) -> Self {
        AdapterError::SerializationError(err.to_string())
    }
}

impl From<std::io::Error> for AdapterError {
    fn from(err: std::io::Error) -> Self {
        AdapterError::IoError(err.to_string())
    }
}
