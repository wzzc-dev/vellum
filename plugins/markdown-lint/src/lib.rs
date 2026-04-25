use vellum_plugin_sdk::decoration::{
    Decoration, DecorationKind, ProtocolResponse, Tooltip, TooltipPosition, UnderlineStyle,
    WebViewRequest,
};
use vellum_plugin_sdk::event::{EventData, EventType};
use vellum_plugin_sdk::ui::{ButtonVariant, Severity, TextStyle, UiEvent, UiNode};
use vellum_plugin_sdk::{Plugin, PluginContext, PluginManifest};

#[derive(Default)]
struct MarkdownLintPlugin {
    diagnostics: Vec<LintDiagnostic>,
    panel_id: u32,
    run_command_id: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct LintDiagnostic {
    line: usize,
    column: usize,
    end_column: usize,
    rule: String,
    message: String,
    severity: Severity,
    fix: Option<LintFix>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct LintFix {
    label: String,
    range_start: usize,
    range_end: usize,
    replacement: String,
}

impl Plugin for MarkdownLintPlugin {
    fn manifest() -> PluginManifest {
        PluginManifest {
            id: "vellum.markdown-lint".into(),
            name: "Markdown Lint".into(),
            version: "0.1.0".into(),
            description: "Checks Markdown documents for common issues".into(),
            author: "Vellum".into(),
        }
    }

    fn init(&mut self, ctx: &mut PluginContext) {
        ctx.subscribe(EventType::DocumentChanged);
        ctx.subscribe(EventType::DocumentOpened);

        self.panel_id = ctx.register_sidebar_panel("markdown-lint", "Lint", "triangle-alert");
        self.run_command_id = ctx.register_command(
            "markdown-lint.run",
            "Run Markdown Lint",
            Some("cmd-shift-l"),
        );
        ctx.register_command("markdown-lint.clear", "Clear Lint Results", None);
    }

    fn handle_event(&mut self, event: EventData, ctx: &mut PluginContext) {
        match event {
            EventData::DocumentChanged { text, .. } => {
                self.run_lint_and_update(&text, ctx);
            }
            EventData::DocumentOpened { .. } => {
                let text = ctx.document_text();
                self.run_lint_and_update(&text, ctx);
            }
            _ => {}
        }
    }

    fn execute_command(&mut self, command_id: u32, ctx: &mut PluginContext) {
        if command_id == self.run_command_id {
            let text = ctx.document_text();
            self.run_lint_and_update(&text, ctx);
        } else if command_id == self.run_command_id + 1 {
            self.diagnostics.clear();
            ctx.clear_decorations();
            self.update_panel_ui(ctx);
        }
    }

    fn handle_ui_event(&mut self, event: UiEvent, ctx: &mut PluginContext) {
        match event {
            UiEvent::ButtonClicked { element_id } => match element_id.as_str() {
                "run-lint" => {
                    let text = ctx.document_text();
                    self.run_lint_and_update(&text, ctx);
                }
                "clear-lint" => {
                    self.diagnostics.clear();
                    ctx.clear_decorations();
                    self.update_panel_ui(ctx);
                }
                id if id.starts_with("fix-") => {
                    if let Some(idx) = id.strip_prefix("fix-").and_then(|s| s.parse::<usize>().ok())
                    {
                        if let Some(diag) = self.diagnostics.get(idx) {
                            if let Some(fix) = &diag.fix {
                                ctx.replace_range(fix.range_start, fix.range_end, &fix.replacement);
                            }
                        }
                    }
                }
                _ => {}
            },
            UiEvent::LinkClicked { element_id } => {
                if let Some(idx) = element_id
                    .strip_prefix("diag-")
                    .and_then(|s| s.parse::<usize>().ok())
                {
                    if let Some(diag) = self.diagnostics.get(idx) {
                        ctx.replace_range(diag.column, diag.column, "");
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_hover(&mut self, hover_data: &str, _ctx: &mut PluginContext) -> Option<Tooltip> {
        let idx: usize = hover_data.parse().ok()?;
        let diag = self.diagnostics.get(idx)?;

        let mut children = vec![
            UiNode::row()
                .gap(6.0)
                .child(UiNode::badge(&diag.rule).severity(diag.severity).build())
                .child(UiNode::styled_text(&diag.message, TextStyle::default().bold()))
                .build(),
            UiNode::styled_text(
                &format!("Line {}, Column {}", diag.line, diag.column),
                TextStyle::small().muted(),
            ),
        ];

        if diag.fix.is_some() {
            children.push(
                UiNode::button(&format!("fix-{}", idx), "Fix")
                    .variant(ButtonVariant::Primary)
                    .build(),
            );
        }

        Some(Tooltip {
            content: UiNode::column()
                .gap(6.0)
                .padding(8.0)
                .children(children)
                .build(),
            position: TooltipPosition::Above,
        })
    }

    fn handle_webview_request(
        &mut self,
        request: WebViewRequest,
        ctx: &mut PluginContext,
    ) -> Option<ProtocolResponse> {
        if request.url.contains("/preview") {
            let text = ctx.document_text();
            let html = markdown_to_html(&text);
            Some(ProtocolResponse {
                mime_type: "text/html".into(),
                body: html.into_bytes(),
            })
        } else {
            None
        }
    }
}

impl MarkdownLintPlugin {
    fn run_lint_and_update(&mut self, text: &str, ctx: &mut PluginContext) {
        self.diagnostics = run_lint(text);
        self.update_decorations(ctx);
        self.update_panel_ui(ctx);

        let error_count = self
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count();
        let warn_count = self
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .count();
        if error_count > 0 || warn_count > 0 {
            ctx.set_status_message(&format!(
                "Lint: {} errors, {} warnings",
                error_count, warn_count
            ));
        } else {
            ctx.set_status_message("Lint: No issues found");
        }
    }

    fn update_decorations(&self, ctx: &mut PluginContext) {
        let decorations: Vec<Decoration> = self
            .diagnostics
            .iter()
            .enumerate()
            .map(|(idx, d)| {
                let (color, style) = match d.severity {
                    Severity::Error => ("red", UnderlineStyle::Wavy),
                    Severity::Warning => ("yellow", UnderlineStyle::Wavy),
                    Severity::Info => ("blue", UnderlineStyle::Dotted),
                    Severity::Hint => ("muted-foreground", UnderlineStyle::Dotted),
                };
                Decoration {
                    id: format!("lint-{}", idx),
                    start: d.column,
                    end: d.end_column,
                    kind: DecorationKind::Underline {
                        color: color.into(),
                        style,
                    },
                    tooltip: Some(format!("{}: {}", d.rule, d.message)),
                    hover_data: Some(idx.to_string()),
                }
            })
            .collect();
        ctx.set_decorations(decorations);
    }

    fn update_panel_ui(&self, ctx: &mut PluginContext) {
        let errors: Vec<_> = self
            .diagnostics
            .iter()
            .enumerate()
            .filter(|(_, d)| d.severity == Severity::Error)
            .collect();
        let warnings: Vec<_> = self
            .diagnostics
            .iter()
            .enumerate()
            .filter(|(_, d)| d.severity == Severity::Warning)
            .collect();
        let infos: Vec<_> = self
            .diagnostics
            .iter()
            .enumerate()
            .filter(|(_, d)| d.severity == Severity::Info || d.severity == Severity::Hint)
            .collect();

        let mut root = UiNode::column().gap(8.0).padding(8.0);

        root = root.child(
            UiNode::row()
                .gap(6.0)
                .child(
                    UiNode::button("run-lint", "Run")
                        .variant(ButtonVariant::Primary)
                        .icon("play")
                        .build(),
                )
                .child(
                    UiNode::button("clear-lint", "Clear")
                        .variant(ButtonVariant::Ghost)
                        .build(),
                )
                .build(),
        );

        if !errors.is_empty() {
            let mut group = UiNode::disclosure(&format!("Errors ({})", errors.len())).open(true);
            for (idx, d) in &errors {
                group = group.child(
                    UiNode::row()
                        .gap(4.0)
                        .child(UiNode::badge(&d.rule).severity(Severity::Error).build())
                        .child(UiNode::link(
                            &format!("diag-{}", idx),
                            &format!("{}: {}", d.line, d.message),
                        ))
                        .build(),
                );
            }
            root = root.child(group.build());
        }

        if !warnings.is_empty() {
            let mut group = UiNode::disclosure(&format!("Warnings ({})", warnings.len()));
            for (idx, d) in &warnings {
                group = group.child(
                    UiNode::row()
                        .gap(4.0)
                        .child(UiNode::badge(&d.rule).severity(Severity::Warning).build())
                        .child(UiNode::link(
                            &format!("diag-{}", idx),
                            &format!("{}: {}", d.line, d.message),
                        ))
                        .build(),
                );
            }
            root = root.child(group.build());
        }

        if !infos.is_empty() {
            let mut group = UiNode::disclosure(&format!("Info ({})", infos.len()));
            for (idx, d) in &infos {
                group = group.child(
                    UiNode::row()
                        .gap(4.0)
                        .child(UiNode::badge(&d.rule).severity(Severity::Info).build())
                        .child(UiNode::link(
                            &format!("diag-{}", idx),
                            &format!("{}: {}", d.line, d.message),
                        ))
                        .build(),
                );
            }
            root = root.child(group.build());
        }

        if self.diagnostics.is_empty() {
            root = root.child(UiNode::styled_text(
                "No issues found",
                TextStyle::default().muted(),
            ));
        }

        root = root.child(
            UiNode::disclosure("Preview")
                .open(false)
                .child(
                    UiNode::webview("md-preview", "https://example.com")
                        .allow_scripts(false)
                        .allow_devtools(false)
                        .build(),
                )
                .build(),
        );

        ctx.set_panel_ui(self.panel_id, root.build());
    }
}

fn run_lint(text: &str) -> Vec<LintDiagnostic> {
    let mut diagnostics = Vec::new();
    let mut prev_line_blank = false;
    let mut in_list = false;
    let mut consecutive_blank = 0;
    let mut byte_offset = 0;

    for (line_idx, line) in text.lines().enumerate() {
        let line_num = line_idx + 1;
        let line_start = byte_offset;

        let trimmed_end = line.trim_end();
        if line.len() > trimmed_end.len() {
            diagnostics.push(LintDiagnostic {
                line: line_num,
                column: line_start + trimmed_end.len(),
                end_column: line_start + line.len(),
                rule: "MD009".into(),
                message: "Trailing spaces".into(),
                severity: Severity::Hint,
                fix: Some(LintFix {
                    label: "Remove trailing spaces".into(),
                    range_start: line_start + trimmed_end.len(),
                    range_end: line_start + line.len(),
                    replacement: String::new(),
                }),
            });
        }

        if line.len() > 120 {
            diagnostics.push(LintDiagnostic {
                line: line_num,
                column: line_start + 120,
                end_column: line_start + line.len(),
                rule: "MD013".into(),
                message: format!("Line length {} exceeds 120", line.len()),
                severity: Severity::Info,
                fix: None,
            });
        }

        if line_idx == 0 && !line.starts_with("# ") {
            diagnostics.push(LintDiagnostic {
                line: 1,
                column: line_start,
                end_column: line_start + line.len().min(1),
                rule: "MD041".into(),
                message: "First line should be a top-level heading".into(),
                severity: Severity::Warning,
                fix: None,
            });
        }

        if line.starts_with('#') && !prev_line_blank && line_idx > 0 {
            diagnostics.push(LintDiagnostic {
                line: line_num,
                column: line_start,
                end_column: line_start + 1,
                rule: "MD022".into(),
                message: "Headings should be surrounded by blank lines".into(),
                severity: Severity::Error,
                fix: Some(LintFix {
                    label: "Insert blank line before heading".into(),
                    range_start: line_start,
                    range_end: line_start,
                    replacement: "\n".into(),
                }),
            });
        }

        let is_list = line.starts_with("- ") || line.starts_with("* ") || line.starts_with("+ ");
        if is_list && !in_list && !prev_line_blank && line_idx > 0 {
            diagnostics.push(LintDiagnostic {
                line: line_num,
                column: line_start,
                end_column: line_start + 2,
                rule: "MD032".into(),
                message: "Lists should be surrounded by blank lines".into(),
                severity: Severity::Warning,
                fix: Some(LintFix {
                    label: "Insert blank line before list".into(),
                    range_start: line_start,
                    range_end: line_start,
                    replacement: "\n".into(),
                }),
            });
        }
        in_list = is_list;

        if line.trim().is_empty() {
            consecutive_blank += 1;
            if consecutive_blank > 1 {
                diagnostics.push(LintDiagnostic {
                    line: line_num,
                    column: line_start,
                    end_column: line_start + 1,
                    rule: "MD012".into(),
                    message: "Multiple consecutive blank lines".into(),
                    severity: Severity::Hint,
                    fix: Some(LintFix {
                        label: "Remove extra blank lines".into(),
                        range_start: line_start - 1,
                        range_end: line_start + 1,
                        replacement: "\n".into(),
                    }),
                });
            }
        } else {
            consecutive_blank = 0;
        }

        prev_line_blank = line.trim().is_empty();
        byte_offset += line.len() + 1;
    }

    diagnostics
}

fn markdown_to_html(text: &str) -> String {
    let mut body = String::new();
    for line in text.lines() {
        if line.starts_with("# ") {
            body.push_str(&format!("<h1>{}</h1>", &line[2..]));
        } else if line.starts_with("## ") {
            body.push_str(&format!("<h2>{}</h2>", &line[3..]));
        } else if line.starts_with("### ") {
            body.push_str(&format!("<h3>{}</h3>", &line[4..]));
        } else if line.trim().is_empty() {
            body.push_str("<br/>");
        } else {
            let escaped = line.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
            body.push_str(&format!("<p>{}</p>", escaped));
        }
    }
    format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"/><style>body{{font-family:sans-serif;padding:12px;font-size:14px;}}</style></head><body>{}</body></html>",
        body
    )
}

vellum_plugin_sdk::vellum_plugin!(MarkdownLintPlugin);
