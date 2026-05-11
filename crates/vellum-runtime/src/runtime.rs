use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::ResourceTable;
use wasmtime_wasi::p2::{IoView, WasiCtx, WasiCtxBuilder, WasiView};

use crate::manifest::VellumManifest;
use crate::ui::{
    AppEvent, ButtonProps, ButtonVariant, CommandEvent, ContainerProps, EdgeInsets, EditorSnapshot,
    InputProps, NativeEvent, NativeViewProps, PluginCommand, PluginInfo, PluginPanel, PluginState,
    Property, ScrollViewProps, SplitAxis, SplitViewProps, TabItem, TabsProps, TextProps, TextStyle,
    UiEvent, ViewKind, ViewNode, ViewTree,
};

#[allow(dead_code)]
mod bindings {
    wasmtime::component::bindgen!({
        path: "../../wit/vellum-app.wit",
        world: "app-world",
    });
}

use bindings::AppWorld;
use bindings::vellum::app::types::{
    AppContext, AppError, AppEvent as WitAppEvent, ButtonProps as WitButtonProps,
    ButtonVariant as WitButtonVariant, CommandEvent as WitCommandEvent,
    ContainerProps as WitContainerProps, EdgeInsets as WitEdgeInsets,
    EditorSnapshot as WitEditorSnapshot, InputProps as WitInputProps, LogLevel,
    NativeEvent as WitNativeEvent, NativeViewProps as WitNativeViewProps,
    PluginCommand as WitPluginCommand, PluginInfo as WitPluginInfo, PluginPanel as WitPluginPanel,
    PluginState as WitPluginState, Property as WitProperty, ScrollViewProps as WitScrollViewProps,
    SplitAxis as WitSplitAxis, SplitViewProps as WitSplitViewProps, TabItem as WitTabItem,
    TabsProps as WitTabsProps, TextProps as WitTextProps, TextStyle as WitTextStyle,
    UiEvent as WitUiEvent, ViewKind as WitViewKind, ViewNode as WitViewNode,
    ViewTree as WitViewTree,
};

pub struct VellumAppRuntime {
    engine: Engine,
    linker: Linker<AppRuntimeState>,
}

impl VellumAppRuntime {
    pub fn new() -> Result<Self> {
        let mut config = Config::new();
        config.wasm_component_model(true);
        let engine = Engine::new(&config)?;
        let mut linker = Linker::new(&engine);
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
        AppWorld::add_to_linker::<AppRuntimeState, wasmtime::component::HasSelf<AppRuntimeState>>(
            &mut linker,
            |state| state,
        )?;
        Ok(Self { engine, linker })
    }

    pub fn load_manifest(&self, directory: impl AsRef<Path>) -> Result<LoadedAppComponent> {
        let directory = directory.as_ref();
        let manifest_path = directory.join("vellum.toml");
        let manifest = VellumManifest::from_toml_bytes(&std::fs::read(&manifest_path)?)
            .with_context(|| format!("failed to parse {}", manifest_path.display()))?;
        self.load_component(directory.to_path_buf(), manifest)
    }

    pub fn load_component(
        &self,
        directory: PathBuf,
        manifest: VellumManifest,
    ) -> Result<LoadedAppComponent> {
        let component_path = directory.join(&manifest.component);
        if !component_path.exists() {
            anyhow::bail!("component not found: {}", component_path.display());
        }
        let component = Component::from_file(&self.engine, &component_path)
            .with_context(|| format!("failed to load component {}", component_path.display()))?;
        let mut store = Store::new(
            &self.engine,
            AppRuntimeState::new(manifest.id.clone(), directory.clone()),
        );
        let bindings = AppWorld::instantiate(&mut store, &component, &self.linker)
            .context("failed to instantiate app component")?;

        Ok(LoadedAppComponent {
            manifest,
            directory,
            store,
            bindings,
            view_tree: None,
        })
    }
}

pub struct LoadedAppComponent {
    manifest: VellumManifest,
    directory: PathBuf,
    store: Store<AppRuntimeState>,
    bindings: AppWorld,
    view_tree: Option<ViewTree>,
}

impl LoadedAppComponent {
    pub fn manifest(&self) -> &VellumManifest {
        &self.manifest
    }

    pub fn view_tree(&self) -> Option<&ViewTree> {
        self.view_tree.as_ref()
    }

    pub fn take_status_message(&mut self) -> Option<String> {
        self.store.data_mut().status_message.take()
    }

    pub fn render_requested(&self) -> bool {
        self.store.data().render_requested
    }

    pub fn clear_render_requested(&mut self) {
        self.store.data_mut().render_requested = false;
    }

    pub fn set_editor_snapshot(&mut self, snapshot: EditorSnapshot) {
        self.store.data_mut().editor_snapshot = snapshot;
    }

    pub fn set_plugin_infos(&mut self, plugins: Vec<PluginInfo>) {
        self.store.data_mut().plugins = plugins;
    }

    pub fn take_editor_commands(&mut self) -> Vec<EditorCommandRequest> {
        std::mem::take(&mut self.store.data_mut().editor_commands)
    }

    pub fn take_plugin_actions(&mut self) -> Vec<PluginAction> {
        std::mem::take(&mut self.store.data_mut().plugin_actions)
    }

    pub fn init(&mut self) -> Result<&ViewTree> {
        let ctx = AppContext {
            app_id: self.manifest.id.clone(),
            app_path: self.directory.to_string_lossy().to_string(),
        };
        let tree = extension_call_result(self.bindings.call_init(&mut self.store, &ctx)?)?;
        self.view_tree = Some(convert_view_tree(tree));
        Ok(self.view_tree.as_ref().unwrap())
    }

    pub fn update(&mut self, event: AppEvent) -> Result<&ViewTree> {
        let event = convert_app_event(event);
        let tree = extension_call_result(self.bindings.call_update(&mut self.store, &event)?)?;
        self.view_tree = Some(convert_view_tree(tree));
        Ok(self.view_tree.as_ref().unwrap())
    }

    pub fn shutdown(&mut self) -> Result<()> {
        extension_call_result(self.bindings.call_shutdown(&mut self.store)?)?;
        Ok(())
    }
}

pub type LoadedComponent = LoadedAppComponent;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorCommandRequest {
    pub command_id: String,
    pub payload: Vec<Property>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginActionKind {
    Enable,
    Disable,
    Reload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginAction {
    pub id: String,
    pub kind: PluginActionKind,
}

pub struct AppRuntimeState {
    app_id: String,
    wasi_ctx: WasiCtx,
    resource_table: ResourceTable,
    status_message: Option<String>,
    render_requested: bool,
    editor_snapshot: EditorSnapshot,
    plugins: Vec<PluginInfo>,
    editor_commands: Vec<EditorCommandRequest>,
    plugin_actions: Vec<PluginAction>,
}

impl AppRuntimeState {
    fn new(app_id: String, app_path: PathBuf) -> Self {
        Self {
            app_id: format!("{}@{}", app_id, app_path.display()),
            wasi_ctx: WasiCtxBuilder::new().build(),
            resource_table: ResourceTable::new(),
            status_message: None,
            render_requested: false,
            editor_snapshot: EditorSnapshot::default(),
            plugins: Vec::new(),
            editor_commands: Vec::new(),
            plugin_actions: Vec::new(),
        }
    }
}

impl IoView for AppRuntimeState {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.resource_table
    }
}

impl WasiView for AppRuntimeState {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi_ctx
    }
}

impl bindings::vellum::app::types::Host for AppRuntimeState {}

impl bindings::vellum::app::host::Host for AppRuntimeState {
    fn log(&mut self, level: LogLevel, message: String) {
        eprintln!("[app:{}:{level:?}] {message}", self.app_id);
    }

    fn show_status_message(&mut self, message: String) -> std::result::Result<(), AppError> {
        self.status_message = Some(message);
        Ok(())
    }

    fn request_render(&mut self) -> std::result::Result<(), AppError> {
        self.render_requested = true;
        Ok(())
    }

    fn editor_command(
        &mut self,
        command_id: String,
        payload: Vec<WitProperty>,
    ) -> std::result::Result<(), AppError> {
        self.editor_commands.push(EditorCommandRequest {
            command_id,
            payload: payload.into_iter().map(convert_property).collect(),
        });
        Ok(())
    }

    fn get_editor_snapshot(&mut self) -> std::result::Result<WitEditorSnapshot, AppError> {
        Ok(convert_editor_snapshot_to_wit(self.editor_snapshot.clone()))
    }

    fn plugin_list(&mut self) -> std::result::Result<Vec<WitPluginInfo>, AppError> {
        Ok(self
            .plugins
            .clone()
            .into_iter()
            .map(convert_plugin_info_to_wit)
            .collect())
    }

    fn plugin_enable(&mut self, id: String) -> std::result::Result<(), AppError> {
        self.plugin_actions.push(PluginAction {
            id,
            kind: PluginActionKind::Enable,
        });
        Ok(())
    }

    fn plugin_disable(&mut self, id: String) -> std::result::Result<(), AppError> {
        self.plugin_actions.push(PluginAction {
            id,
            kind: PluginActionKind::Disable,
        });
        Ok(())
    }

    fn plugin_reload(&mut self, id: String) -> std::result::Result<(), AppError> {
        self.plugin_actions.push(PluginAction {
            id,
            kind: PluginActionKind::Reload,
        });
        Ok(())
    }
}

fn extension_call_result<T>(result: std::result::Result<T, AppError>) -> Result<T> {
    result.map_err(|err| anyhow::anyhow!(err.message))
}

fn convert_view_tree(tree: WitViewTree) -> ViewTree {
    ViewTree {
        root: tree.root,
        nodes: tree.nodes.into_iter().map(convert_view_node).collect(),
    }
}

fn convert_view_node(node: WitViewNode) -> ViewNode {
    ViewNode {
        id: node.id,
        kind: convert_view_kind(node.kind),
        children: node.children,
    }
}

fn convert_view_kind(kind: WitViewKind) -> ViewKind {
    match kind {
        WitViewKind::Empty => ViewKind::Empty,
        WitViewKind::Column(props) => ViewKind::Column(convert_container_props(props)),
        WitViewKind::Row(props) => ViewKind::Row(convert_container_props(props)),
        WitViewKind::Text(props) => ViewKind::Text(convert_text_props(props)),
        WitViewKind::Button(props) => ViewKind::Button(convert_button_props(props)),
        WitViewKind::Input(props) => ViewKind::Input(convert_input_props(props)),
        WitViewKind::Tabs(props) => ViewKind::Tabs(convert_tabs_props(props)),
        WitViewKind::SplitView(props) => ViewKind::SplitView(convert_split_view_props(props)),
        WitViewKind::ScrollView(props) => ViewKind::ScrollView(convert_scroll_view_props(props)),
        WitViewKind::NativeView(props) => ViewKind::NativeView(convert_native_view_props(props)),
    }
}

fn convert_container_props(props: WitContainerProps) -> ContainerProps {
    ContainerProps {
        gap: props.gap,
        padding: props.padding.map(convert_edge_insets),
    }
}

fn convert_edge_insets(insets: WitEdgeInsets) -> EdgeInsets {
    EdgeInsets {
        top: insets.top,
        right: insets.right,
        bottom: insets.bottom,
        left: insets.left,
    }
}

fn convert_text_props(props: WitTextProps) -> TextProps {
    TextProps {
        content: props.content,
        style: convert_text_style(props.style),
    }
}

fn convert_text_style(style: WitTextStyle) -> TextStyle {
    TextStyle {
        size: style.size,
        color: style.color,
        bold: style.bold,
        italic: style.italic,
        monospace: style.monospace,
    }
}

fn convert_button_props(props: WitButtonProps) -> ButtonProps {
    ButtonProps {
        label: props.label,
        style: convert_button_variant(props.style),
        disabled: props.disabled,
    }
}

fn convert_button_variant(variant: WitButtonVariant) -> ButtonVariant {
    match variant {
        WitButtonVariant::Primary => ButtonVariant::Primary,
        WitButtonVariant::Secondary => ButtonVariant::Secondary,
        WitButtonVariant::Ghost => ButtonVariant::Ghost,
        WitButtonVariant::Danger => ButtonVariant::Danger,
    }
}

fn convert_input_props(props: WitInputProps) -> InputProps {
    InputProps {
        placeholder: props.placeholder,
        value: props.value,
        single_line: props.single_line,
    }
}

fn convert_tabs_props(props: WitTabsProps) -> TabsProps {
    TabsProps {
        selected: props.selected,
        tabs: props.tabs.into_iter().map(convert_tab_item).collect(),
    }
}

fn convert_tab_item(item: WitTabItem) -> TabItem {
    TabItem {
        id: item.id,
        label: item.label,
        child: item.child,
    }
}

fn convert_split_view_props(props: WitSplitViewProps) -> SplitViewProps {
    SplitViewProps {
        axis: convert_split_axis(props.axis),
        ratio: props.ratio,
    }
}

fn convert_scroll_view_props(props: WitScrollViewProps) -> ScrollViewProps {
    ScrollViewProps {
        axis: convert_split_axis(props.axis),
    }
}

fn convert_split_axis(axis: WitSplitAxis) -> SplitAxis {
    match axis {
        WitSplitAxis::Horizontal => SplitAxis::Horizontal,
        WitSplitAxis::Vertical => SplitAxis::Vertical,
    }
}

fn convert_native_view_props(props: WitNativeViewProps) -> NativeViewProps {
    NativeViewProps {
        kind: props.kind,
        props: props.props.into_iter().map(convert_property).collect(),
    }
}

fn convert_property(prop: WitProperty) -> Property {
    Property {
        name: prop.name,
        value: prop.value,
    }
}

fn convert_app_event(event: AppEvent) -> WitAppEvent {
    match event {
        AppEvent::Ui(event) => WitAppEvent::Ui(convert_ui_event(event)),
        AppEvent::Native(event) => WitAppEvent::Native(convert_native_event(event)),
        AppEvent::Command(event) => WitAppEvent::Command(convert_command_event(event)),
        AppEvent::Tick(value) => WitAppEvent::Tick(value),
    }
}

fn convert_ui_event(event: UiEvent) -> WitUiEvent {
    WitUiEvent {
        target_id: event.target_id,
        event_kind: event.event_kind,
        value: event.value,
        index: event.index,
        checked: event.checked,
    }
}

fn convert_native_event(event: NativeEvent) -> WitNativeEvent {
    WitNativeEvent {
        view_id: event.view_id,
        event_kind: event.event_kind,
        payload: event
            .payload
            .into_iter()
            .map(convert_property_to_wit)
            .collect(),
    }
}

fn convert_command_event(event: CommandEvent) -> WitCommandEvent {
    WitCommandEvent {
        command_id: event.command_id,
        payload: event
            .payload
            .into_iter()
            .map(convert_property_to_wit)
            .collect(),
    }
}

fn convert_property_to_wit(prop: Property) -> WitProperty {
    WitProperty {
        name: prop.name,
        value: prop.value,
    }
}

fn convert_editor_snapshot_to_wit(snapshot: EditorSnapshot) -> WitEditorSnapshot {
    WitEditorSnapshot {
        display_name: snapshot.display_name,
        path: snapshot.path,
        dirty: snapshot.dirty,
        word_count: snapshot.word_count,
        document_text: snapshot.document_text,
        view_mode: snapshot.view_mode,
    }
}

fn convert_plugin_info_to_wit(info: PluginInfo) -> WitPluginInfo {
    WitPluginInfo {
        id: info.id,
        name: info.name,
        version: info.version,
        description: info.description,
        state: convert_plugin_state_to_wit(info.state),
        commands: info
            .commands
            .into_iter()
            .map(convert_plugin_command_to_wit)
            .collect(),
        panels: info
            .panels
            .into_iter()
            .map(convert_plugin_panel_to_wit)
            .collect(),
        error: info.error,
    }
}

fn convert_plugin_state_to_wit(state: PluginState) -> WitPluginState {
    match state {
        PluginState::Enabled => WitPluginState::Enabled,
        PluginState::Disabled => WitPluginState::Disabled,
        PluginState::Failed => WitPluginState::Failed,
    }
}

fn convert_plugin_command_to_wit(command: PluginCommand) -> WitPluginCommand {
    WitPluginCommand {
        id: command.id,
        title: command.title,
    }
}

fn convert_plugin_panel_to_wit(panel: PluginPanel) -> WitPluginPanel {
    WitPluginPanel {
        id: panel.id,
        title: panel.title,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_initializes() {
        assert!(VellumAppRuntime::new().is_ok());
    }
}
