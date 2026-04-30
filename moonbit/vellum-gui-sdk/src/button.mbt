// Button widget for Vellum GUI Framework

pub type Button = {
  pub @types.Widget
  label: String,
  icon: Option[String],
  variant: ButtonVariant,
  disabled: Bool,
  on_click: Option[() -> Void],
}

pub enum ButtonVariant {
  Primary,
  Secondary,
  Ghost,
  Danger,
}

pub fn Button::new(label: String) -> Button {
  {
    widget: {
      id: @types.generate_id(),
      widget_type: "button",
      children: [{
        id: @types.generate_id(),
        widget_type: "button_text",
        children: [],
        layout: @types.layout_default(),
        props: {
          @types.props_default() with
          text_content: label,
        },
      }],
      layout: @types.layout_default(),
      props: {
        @types.props_default() with
        button_label: label,
        button_variant: "primary",
      },
    },
    label,
    icon: None,
    variant: Primary,
    disabled: false,
    on_click: None,
  }
}

pub fn Button::icon(self: Button, icon_name: String) -> Button {
  { icon: Some(icon_name), ..self }
}

pub fn Button::variant(self: Button, variant: ButtonVariant) -> Button {
  {
    variant,
    widget: {
      ..self.widget with
      props: {
        self.widget.props with
        button_variant: match variant {
          Primary => "primary"
          Secondary => "secondary"
          Ghost => "ghost"
          Danger => "danger"
        }
      }
    }
  }
}

pub fn Button::disabled(self: Button, disabled: Bool) -> Button {
  { disabled, ..self }
}

pub fn Button::on_click(self: Button, handler: () -> Void) -> Button {
  { on_click: Some(handler), ..self }
}

// Icon button
pub type IconButton = {
  pub @types.Widget
  icon: String,
  size: Float,
  variant: ButtonVariant,
  disabled: Bool,
  on_click: Option[() -> Void],
}

pub fn IconButton::new(icon: String) -> IconButton {
  {
    widget: {
      id: @types.generate_id(),
      widget_type: "icon_button",
      children: [],
      layout: @types.layout_default(),
      props: {
        @types.props_default() with
        icon_name: icon,
        icon_size: 24.0,
      },
    },
    icon,
    size: 24.0,
    variant: Secondary,
    disabled: false,
    on_click: None,
  }
}

pub fn IconButton::size(self: IconButton, size: Float) -> IconButton {
  {
    size,
    widget: {
      ..self.widget with
      props: {
        self.widget.props with
        icon_size: size
      }
    }
  }
}

pub fn IconButton::variant(self: IconButton, variant: ButtonVariant) -> IconButton {
  { variant, ..self }
}

pub fn IconButton::on_click(self: IconButton, handler: () -> Void) -> IconButton {
  { on_click: Some(handler), ..self }
}

// Text button
pub type TextButton = {
  pub @types.Widget
  label: String,
  variant: TextButtonVariant,
  disabled: Bool,
  on_click: Option[() -> Void],
}

pub enum TextButtonVariant {
  Body1,
  Body2,
  Caption,
  Overline,
}

pub fn TextButton::new(label: String) -> TextButton {
  {
    widget: {
      id: @types.generate_id(),
      widget_type: "text_button",
      children: [{
        id: @types.generate_id(),
        widget_type: "button_text",
        children: [],
        layout: @types.layout_default(),
        props: {
          @types.props_default() with
          text_content: label,
          text_variant: "body1",
        },
      }],
      layout: @types.layout_default(),
      props: {
        @types.props_default() with
        button_label: label,
      },
    },
    label,
    variant: Body1,
    disabled: false,
    on_click: None,
  }
}

pub fn TextButton::variant(self: TextButton, variant: TextButtonVariant) -> TextButton {
  { variant, ..self }
}

pub fn TextButton::on_click(self: TextButton, handler: () -> Void) -> TextButton {
  { on_click: Some(handler), ..self }
}

// Link button
pub type LinkButton = {
  pub @types.Widget
  label: String,
  disabled: Bool,
  on_click: Option[() -> Void],
}

pub fn LinkButton::new(label: String) -> LinkButton {
  {
    widget: {
      id: @types.generate_id(),
      widget_type: "link_button",
      children: [{
        id: @types.generate_id(),
        widget_type: "link_text",
        children: [],
        layout: @types.layout_default(),
        props: {
          @types.props_default() with
          text_content: label,
          text_decoration: "underline",
        },
      }],
      layout: @types.layout_default(),
      props: {
        @types.props_default() with
        button_label: label,
      },
    },
    label,
    disabled: false,
    on_click: None,
  }
}

pub fn LinkButton::on_click(self: LinkButton, handler: () -> Void) -> LinkButton {
  { on_click: Some(handler), ..self }
}

// FAB (Floating Action Button)
pub type FAB = {
  pub @types.Widget
  icon: String,
  size: FABSize,
  extended: Bool,
  label: Option[String],
  on_click: Option[() -> Void],
}

pub enum FABSize {
  Small,
  Medium,
  Large,
}

pub fn FAB::new(icon: String) -> FAB {
  {
    widget: {
      id: @types.generate_id(),
      widget_type: "fab",
      children: [{
        id: @types.generate_id(),
        widget_type: "fab_icon",
        children: [],
        layout: @types.layout_default(),
        props: {
          @types.props_default() with
          icon_name: icon,
          icon_size: 24.0,
        },
      }],
      layout: @types.layout_default(),
      props: {
        @types.props_default() with
        icon_name: icon,
        fab_size: "medium",
      },
    },
    icon,
    size: Medium,
    extended: false,
    label: None,
    on_click: None,
  }
}

pub fn FAB::size(self: FAB, size: FABSize) -> FAB {
  { size, ..self }
}

pub fn FAB::extended(self: FAB, label: String) -> FAB {
  {
    extended: true,
    label: Some(label),
    widget: {
      ..self.widget with
      children: @types.concat(
        self.widget.children,
        [{
          id: @types.generate_id(),
          widget_type: "fab_label",
          children: [],
          layout: @types.layout_default(),
          props: {
            @types.props_default() with
            text_content: label,
          },
        }]
      )
    }
  }
}

pub fn FAB::on_click(self: FAB, handler: () -> Void) -> FAB {
  { on_click: Some(handler), ..self }
}
