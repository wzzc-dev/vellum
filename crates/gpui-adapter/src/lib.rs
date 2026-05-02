pub mod bridge;
pub mod error;
pub mod event;
pub mod gpui_render;
pub mod paint;
pub mod types;
pub mod widget;
pub mod window;

#[cfg(feature = "wit")]
pub mod wit_host;

// Navigation rendering module
pub mod navigation_render;

// Animation rendering module
pub mod animation_render;

// Grid rendering module
pub mod grid_render;

// Environment rendering module
pub mod environment_render;

pub use bridge::GpuiBridge;
pub use error::{AdapterError, Result};
pub use event::{EventDispatcher, EventType, GpuiEvent, MouseButton, MouseEventKind};
pub use paint::{Canvas, Image};
pub use types::{
    Alignment, AppTheme, Border, BoxShadow, Color, CursorShape, EdgeInsets, FlexDirection,
    FlexParams, FontStyle, FontWeight, Point, Rect, Size, TextAlign, TextDecoration, TextStyle,
    VerticalAlign, Visibility, WidgetDisplay, WidgetLayout, WidgetPosition, Wrap,
};
pub use widget::{Widget, WidgetId, WidgetManager};
pub use window::{Window, WindowId, WindowManager, WindowOptions};

#[cfg(feature = "wit")]
pub use wit_host::{GuiHost, GuiRuntimeState};

// Re-export navigation types
pub use navigation_render::{
    NavigatorController, NavigationController as NavControllerResource,
    NavigationControllerResource, NavigationEvent, NavigationState, RouteConfig,
    TabBar, TabBarItem, TransitionType,
};

// Re-export animation types
pub use animation_render::{
    AnimationController, AnimationCurve, AnimationDirection, AnimationEvent,
    AnimationSpec, AnimationStatus, Keyframe, PageTransition, SlideDirection,
    SpringConfig,
};

// Re-export grid types
pub use grid_render::{
    GridAlignment, GridConfig, GridEvent, GridItemData, GridItemPlacement,
    GridLinePlacement, GridTrackSize, GridView as GridViewController, LazyGridConfig,
    LazyGridView, ColumnSpan, RowSpan,
};

// Re-export environment types
pub use environment_render::{
    ColorScheme, EnvChangeEvent, EnvModifier, EnvReader, EnvSnapshot, EnvValue,
    EnvironmentProvider, TextDirection,
};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
