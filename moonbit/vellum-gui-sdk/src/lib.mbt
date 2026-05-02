///|
/// Vellum GUI Framework
/// A declarative UI framework with reactive state management.

///|
/// Re-export types from the state package.
pub using @state {
  type Observable,
  type State,
  type Binding,
  type Widget,
  type Container,
  type ContainerType,
  type Text,
  type Button,
  type ButtonType,
  type TextInput,
  type Checkbox,
  type Toggle,
  type Select,
  type SelectOption,
  type Slider,
  type ProgressBar,
  type ScrollView,
  type ScrollDirection,
  type Color,
  type Border,
  type Shadow,
  type CursorType,
  type Style,
  type EdgeInsets,
  type Constraints,
  type Alignment,
  type Layout,
  type AppOptions,
  type WindowOptions,
  type Window,
  type App,
  type EventEmitter,
  type MouseEventType,
  type KeyEventType,
  type MouseEvent,
  type KeyEvent,
}

///|
/// Re-export navigation types.
pub using @navigation {
  type Navigator,
  type NavigationState,
  type NavigationParams,
  type TransitionType,
  type RouteConfig,
  type NavHostConfig,
  type NavigationController,
  type NavigationEvent,
  type TabBarItem,
  type TabBar,
}

///|
/// Re-export animation types.
pub using @animation {
  type AnimationCurve,
  type AnimationDirection,
  type AnimationStatus,
  type AnimationSpec,
  type SpringConfig,
  type AnimatedProperty,
  type Keyframe,
  type KeyframeAnimationConfig,
  type FillMode,
  type AnimationController,
  type AnimationEvent,
  type SlideDirection,
  type PageTransition,
}

///|
/// Re-export grid types.
pub using @grid {
  type GridAlignment,
  type GridTrackSize,
  type GridLinePlacement,
  type ColumnSpan,
  type RowSpan,
  type GridItemPlacement,
  type GridItem,
  type GridConfig,
  type GridView,
}

///|
/// Re-export environment types.
pub using @environment {
  type EnvValue,
  type EnvChangeEvent,
  type EnvironmentProvider,
  type EnvironmentReader,
  type EnvironmentModifier,
  type ColorScheme,
  type TextDirection,
}
