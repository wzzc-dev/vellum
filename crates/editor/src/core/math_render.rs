use std::collections::HashMap;
use std::hash::Hash;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MathRenderMode {
    Inline,
    Display,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MathRenderCacheKey {
    source: String,
    mode: MathRenderMode,
    font_size_bits: u32,
}

impl MathRenderCacheKey {
    pub fn new(source: &str, mode: MathRenderMode, font_size: f32) -> Self {
        Self {
            source: source.to_string(),
            mode,
            font_size_bits: font_size.to_bits(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MathRenderCache {
    entries: HashMap<MathRenderCacheKey, MathNodeTree>,
}

impl MathRenderCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn get_or_parse(&mut self, source: &str, mode: MathRenderMode, font_size: f32) -> MathNodeTree {
        let key = MathRenderCacheKey::new(source, mode, font_size);
        if let Some(tree) = self.entries.get(&key) {
            return tree.clone();
        }
        let tree = parse_math(source);
        self.entries.insert(key, tree.clone());
        tree
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MathNodeTree {
    pub nodes: Vec<MathNode>,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MathNode {
    Text(String),
    Symbol(String),
    Group(Vec<MathNode>),
    Superscript {
        base: Box<MathNode>,
        sup: Box<MathNode>,
    },
    Subscript {
        base: Box<MathNode>,
        sub: Box<MathNode>,
    },
    Fraction {
        numerator: Box<MathNode>,
        denominator: Box<MathNode>,
    },
    Sqrt {
        content: Box<MathNode>,
        index: Option<Box<MathNode>>,
    },
    Overline {
        content: Box<MathNode>,
    },
    Matrix {
        rows: Vec<Vec<MathNode>>,
    },
}

pub fn parse_math(source: &str) -> MathNodeTree {
    let trimmed = source.trim();
    let nodes = parse_nodes(trimmed, 0).0;
    MathNodeTree {
        nodes,
        source: source.to_string(),
    }
}

type ParseResult = (Vec<MathNode>, usize);

fn parse_nodes(input: &str, start: usize) -> ParseResult {
    let mut nodes = Vec::new();
    let mut i = start;
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();

    while i < len {
        let ch = chars[i];

        if ch == '}' {
            break;
        }

        if ch == '^' || ch == '_' {
            if let Some(last) = nodes.pop() {
                i += 1;
                let (arg, end) = parse_single_arg(&chars, i);
                i = end;
                if ch == '^' {
                    nodes.push(MathNode::Superscript {
                        base: Box::new(last),
                        sup: Box::new(arg),
                    });
                } else {
                    nodes.push(MathNode::Subscript {
                        base: Box::new(last),
                        sub: Box::new(arg),
                    });
                }
            } else {
                nodes.push(MathNode::Text(ch.to_string()));
                i += 1;
            }
            continue;
        }

        if ch == '\\' {
            let (cmd_nodes, end) = parse_command(&chars, i);
            nodes.extend(cmd_nodes);
            i = end;
            continue;
        }

        if ch == '{' {
            let (group_node, end) = parse_brace_group(&chars, i);
            nodes.push(group_node);
            i = end;
            continue;
        }

        if ch.is_whitespace() {
            let mut end = i + 1;
            while end < len && chars[end].is_whitespace() {
                end += 1;
            }
            if !nodes.is_empty() && i > start {
                nodes.push(MathNode::Text(" ".to_string()));
            }
            i = end;
            continue;
        }

        if let Some(last) = nodes.last_mut() {
            if let MathNode::Text(s) = last {
                s.push(ch);
                i += 1;
                continue;
            }
        }

        nodes.push(MathNode::Text(ch.to_string()));
        i += 1;
    }

    (nodes, i)
}

fn parse_single_arg(chars: &[char], start: usize) -> (MathNode, usize) {
    if start >= chars.len() {
        return (MathNode::Text(String::new()), start);
    }

    if chars[start] == '{' {
        let (group_node, end) = parse_brace_group(chars, start);
        return (group_node, end);
    }

    let ch = chars[start];
    if ch == '\\' {
        let (nodes, end) = parse_command(chars, start);
        if nodes.len() == 1 {
            (nodes.into_iter().next().unwrap(), end)
        } else {
            (MathNode::Group(nodes), end)
        }
    } else {
        (MathNode::Text(ch.to_string()), start + 1)
    }
}

fn parse_brace_group(chars: &[char], start: usize) -> (MathNode, usize) {
    debug_assert!(chars[start] == '{');
    let input: String = chars.iter().collect();
    let (nodes, end) = parse_nodes(&input, start + 1);
    let end = if end < chars.len() && chars[end] == '}' {
        end + 1
    } else {
        end
    };
    if nodes.len() == 1 {
        (nodes.into_iter().next().unwrap(), end)
    } else {
        (MathNode::Group(nodes), end)
    }
}

fn parse_command(chars: &[char], start: usize) -> (Vec<MathNode>, usize) {
    debug_assert!(chars[start] == '\\');
    let mut i = start + 1;
    let len = chars.len();

    let mut cmd = String::new();
    while i < len && chars[i].is_ascii_alphabetic() {
        cmd.push(chars[i]);
        i += 1;
    }

    if cmd.is_empty() {
        if i < len {
            let ch = chars[i];
            i += 1;
            let escaped = match ch {
                '\\' => "\\",
                '{' => "{",
                '}' => "}",
                '%' => "%",
                '#' => "#",
                '&' => "&",
                '_' => "_",
                '^' => "^",
                '~' => "\u{00A0}",
                ' ' => " ",
                ',' => " ",
                ';' => "  ",
                ':' => " ",
                '|' => "‖",
                '<' => "⟨",
                '>' => "⟩",
                _ => return (vec![MathNode::Symbol(ch.to_string())], i),
            };
            return (vec![MathNode::Text(escaped.to_string())], i);
        }
        return (vec![MathNode::Text("\\".to_string())], i);
    }

    while i < len && chars[i] == ' ' {
        i += 1;
    }

    match cmd.as_str() {
        "frac" => {
            let (num, end1) = parse_single_arg(chars, i);
            let (den, end2) = parse_single_arg(chars, end1);
            (
                vec![MathNode::Fraction {
                    numerator: Box::new(num),
                    denominator: Box::new(den),
                }],
                end2,
            )
        }
        "sqrt" => {
            if i < len && chars[i] == '[' {
                let input: String = chars.iter().collect();
                let bracket_end = input[i..].find(']').map(|p| i + p).unwrap_or(len);
                let index_source: String = chars[i + 1..bracket_end].iter().collect();
                let index_tree = parse_math(&index_source);
                let index_node = if index_tree.nodes.len() == 1 {
                    index_tree.nodes.into_iter().next().unwrap()
                } else {
                    MathNode::Group(index_tree.nodes)
                };
                let (content, end) = parse_single_arg(chars, bracket_end + 1);
                (
                    vec![MathNode::Sqrt {
                        content: Box::new(content),
                        index: Some(Box::new(index_node)),
                    }],
                    end,
                )
            } else {
                let (content, end) = parse_single_arg(chars, i);
                (
                    vec![MathNode::Sqrt {
                        content: Box::new(content),
                        index: None,
                    }],
                    end,
                )
            }
        }
        "overline" | "bar" => {
            let (content, end) = parse_single_arg(chars, i);
            (
                vec![MathNode::Overline {
                    content: Box::new(content),
                }],
                end,
            )
        }
        "text" | "mathrm" | "textup" => {
            let (content, end) = parse_single_arg(chars, i);
            let text = node_to_text(&content);
            (vec![MathNode::Text(text)], end)
        }
        "begin" => {
            let env_name = parse_env_name(chars, i);
            let (env_end, rows) = parse_env_body(chars, i, &env_name);
            (vec![MathNode::Matrix { rows }], env_end)
        }
        "left" | "right" | "bigl" | "bigr" | "Bigl" | "Bigr" | "biggl" | "biggr" => {
            if i < len {
                let (delim_nodes, end) = parse_command_or_char(chars, i);
                (delim_nodes, end)
            } else {
                (vec![], i)
            }
        }
        "limits" | "nolimits" | "displaystyle" | "textstyle" | "scriptstyle"
        | "scriptscriptstyle" => (vec![], i),
        _ => {
            if let Some(symbol) = lookup_symbol(&cmd) {
                (vec![MathNode::Symbol(symbol.to_string())], i)
            } else {
                (vec![MathNode::Text(format!("\\{cmd}"))], i)
            }
        }
    }
}

fn parse_command_or_char(chars: &[char], start: usize) -> (Vec<MathNode>, usize) {
    if start >= chars.len() {
        return (vec![], start);
    }
    if chars[start] == '\\' {
        parse_command(chars, start)
    } else {
        let ch = chars[start];
        match ch {
            '(' => (vec![MathNode::Symbol("(".to_string())], start + 1),
            ')' => (vec![MathNode::Symbol(")".to_string())], start + 1),
            '[' => (vec![MathNode::Symbol("[".to_string())], start + 1),
            ']' => (vec![MathNode::Symbol("]".to_string())], start + 1),
            '|' => (vec![MathNode::Symbol("|".to_string())], start + 1),
            '.' => (vec![], start + 1),
            '<' => (vec![MathNode::Symbol("⟨".to_string())], start + 1),
            '>' => (vec![MathNode::Symbol("⟩".to_string())], start + 1),
            _ => (vec![MathNode::Text(ch.to_string())], start + 1),
        }
    }
}

fn parse_env_name(chars: &[char], start: usize) -> String {
    let mut i = start;
    let len = chars.len();
    while i < len && chars[i] == ' ' {
        i += 1;
    }
    if i < len && chars[i] == '{' {
        i += 1;
        let mut name = String::new();
        while i < len && chars[i] != '}' {
            name.push(chars[i]);
            i += 1;
        }
        name.trim().to_string()
    } else {
        String::new()
    }
}

fn parse_env_body(chars: &[char], start: usize, _env_name: &str) -> (usize, Vec<Vec<MathNode>>) {
    let input: String = chars.iter().collect();
    let mut i = start;
    let len = chars.len();

    while i < len {
        if chars[i] == '\\' {
            let rest = &input[i..];
            if rest.starts_with("\\end") {
                let mut j = i + 4;
                while j < len && chars[j] == ' ' {
                    j += 1;
                }
                if j < len && chars[j] == '{' {
                    let brace_close = input[j..].find('}').map(|p| j + p + 1).unwrap_or(len);
                    i = brace_close;
                    break;
                }
            }
        }
        i += 1;
    }

    let body_start = start;
    let body: String = chars[body_start..i].iter().collect();
    let body = body.trim_end();

    let body_without_end = if let Some(pos) = body.rfind("\\end") {
        &body[..pos]
    } else {
        body
    };

    let mut rows = Vec::new();
    for row_str in body_without_end.split('\\') {
        let row_str = row_str.trim();
        if row_str.is_empty() {
            continue;
        }
        let mut cells = Vec::new();
        for cell_str in row_str.split('&') {
            let cell_str = cell_str.trim();
            if cell_str.is_empty() {
                cells.push(MathNode::Text(String::new()));
                continue;
            }
            let tree = parse_math(cell_str);
            if tree.nodes.len() == 1 {
                cells.push(tree.nodes.into_iter().next().unwrap());
            } else {
                cells.push(MathNode::Group(tree.nodes));
            }
        }
        if !cells.is_empty() {
            rows.push(cells);
        }
    }

    if rows.is_empty() {
        rows.push(vec![MathNode::Text(String::new())]);
    }

    (i, rows)
}

pub fn node_to_text(node: &MathNode) -> String {
    match node {
        MathNode::Text(s) => s.clone(),
        MathNode::Symbol(s) => s.clone(),
        MathNode::Group(nodes) => nodes.iter().map(node_to_text).collect(),
        MathNode::Superscript { base, sup } => {
            let base_text = node_to_text(base);
            let sup_text = node_to_text(sup);
            let unicode_sup = to_unicode_superscript(&sup_text);
            if unicode_sup.is_some() {
                format!("{}{}", base_text, unicode_sup.unwrap())
            } else {
                format!("{}^{}", base_text, sup_text)
            }
        }
        MathNode::Subscript { base, sub } => {
            let base_text = node_to_text(base);
            let sub_text = node_to_text(sub);
            let unicode_sub = to_unicode_subscript(&sub_text);
            if unicode_sub.is_some() {
                format!("{}{}", base_text, unicode_sub.unwrap())
            } else {
                format!("{}_{}", base_text, sub_text)
            }
        }
        MathNode::Fraction { numerator, denominator } => {
            let num_text = node_to_text(numerator);
            let den_text = node_to_text(denominator);
            format!("{}/{}", num_text, den_text)
        }
        MathNode::Sqrt { content, index } => {
            let text = node_to_text(content);
            let index_text = index.as_ref().map(|n| node_to_text(n)).unwrap_or_default();
            if index_text.is_empty() {
                format!("√({})", text)
            } else {
                format!("{}√({})", index_text, text)
            }
        }
        MathNode::Overline { content } => {
            let text = node_to_text(content);
            format!("‾{}", text)
        }
        MathNode::Matrix { rows } => {
            let row_strs: Vec<String> = rows
                .iter()
                .map(|row| {
                    row.iter()
                        .map(node_to_text)
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .collect();
            row_strs.join("; ")
        }
    }
}

fn to_unicode_superscript(text: &str) -> Option<String> {
    let mut result = String::new();
    for ch in text.chars() {
        let sup = match ch {
            '0' => '⁰', '1' => '¹', '2' => '²', '3' => '³', '4' => '⁴',
            '5' => '⁵', '6' => '⁶', '7' => '⁷', '8' => '⁸', '9' => '⁹',
            '+' => '⁺', '-' => '⁻', '=' => '⁼', '(' => '⁽', ')' => '⁾',
            'n' => 'ⁿ', 'i' => 'ⁱ',
            _ => return None,
        };
        result.push(sup);
    }
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

fn to_unicode_subscript(text: &str) -> Option<String> {
    let mut result = String::new();
    for ch in text.chars() {
        let sub = match ch {
            '0' => '₀', '1' => '₁', '2' => '₂', '3' => '₃', '4' => '₄',
            '5' => '₅', '6' => '₆', '7' => '₇', '8' => '₈', '9' => '₉',
            '+' => '₊', '-' => '₋', '=' => '₌', '(' => '₍', ')' => '₎',
            'a' => 'ₐ', 'e' => 'ₑ', 'h' => 'ₕ', 'i' => 'ᵢ', 'o' => 'ₒ',
            'r' => 'ᵣ', 's' => 'ₛ', 't' => 'ₜ', 'x' => 'ₓ',
            _ => return None,
        };
        result.push(sub);
    }
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

pub fn math_tree_to_display_text(tree: &MathNodeTree) -> String {
    if tree.nodes.is_empty() {
        return String::new();
    }
    tree.nodes.iter().map(node_to_text).collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathTokenType {
    Command,
    Brace,
    Number,
    Text,
    Delimiter,
}

#[derive(Debug, Clone)]
pub struct MathToken {
    pub text: String,
    pub token_type: MathTokenType,
}

pub fn highlight_math_source(source: &str) -> Vec<MathToken> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = source.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let ch = chars[i];

        if ch == '\\' && i + 1 < len {
            let cmd_start = i;
            i += 1;
            if chars[i].is_ascii_alphabetic() {
                while i < len && chars[i].is_ascii_alphabetic() {
                    i += 1;
                }
                let cmd_text: String = chars[cmd_start..i].iter().collect();
                tokens.push(MathToken {
                    text: cmd_text,
                    token_type: MathTokenType::Command,
                });
            } else {
                i += 1;
                let esc_text: String = chars[cmd_start..i].iter().collect();
                tokens.push(MathToken {
                    text: esc_text,
                    token_type: MathTokenType::Command,
                });
            }
            continue;
        }

        if ch == '{' || ch == '}' {
            tokens.push(MathToken {
                text: ch.to_string(),
                token_type: MathTokenType::Brace,
            });
            i += 1;
            continue;
        }

        if ch.is_ascii_digit() {
            let num_start = i;
            while i < len && chars[i].is_ascii_digit() || chars[i] == '.' {
                i += 1;
            }
            let num_text: String = chars[num_start..i].iter().collect();
            tokens.push(MathToken {
                text: num_text,
                token_type: MathTokenType::Number,
            });
            continue;
        }

        if ch == '$' || ch == '&' || ch == '#' || ch == '%' || ch == '^' || ch == '_' {
            tokens.push(MathToken {
                text: ch.to_string(),
                token_type: MathTokenType::Delimiter,
            });
            i += 1;
            continue;
        }

        let text_start = i;
        while i < len
            && chars[i] != '\\'
            && chars[i] != '{'
            && chars[i] != '}'
            && !chars[i].is_ascii_digit()
            && chars[i] != '$'
            && chars[i] != '&'
            && chars[i] != '#'
            && chars[i] != '%'
            && chars[i] != '^'
            && chars[i] != '_'
        {
            i += 1;
        }
        if i > text_start {
            let text: String = chars[text_start..i].iter().collect();
            tokens.push(MathToken {
                text,
                token_type: MathTokenType::Text,
            });
        }
    }

    tokens
}

fn lookup_symbol(cmd: &str) -> Option<&'static str> {
    match cmd {
        "alpha" => Some("α"),
        "beta" => Some("β"),
        "gamma" => Some("γ"),
        "delta" => Some("δ"),
        "epsilon" => Some("ε"),
        "varepsilon" => Some("ε"),
        "zeta" => Some("ζ"),
        "eta" => Some("η"),
        "theta" => Some("θ"),
        "vartheta" => Some("ϑ"),
        "iota" => Some("ι"),
        "kappa" => Some("κ"),
        "lambda" => Some("λ"),
        "mu" => Some("μ"),
        "nu" => Some("ν"),
        "xi" => Some("ξ"),
        "omicron" => Some("ο"),
        "pi" => Some("π"),
        "varpi" => Some("ϖ"),
        "rho" => Some("ρ"),
        "varrho" => Some("ϱ"),
        "sigma" => Some("σ"),
        "varsigma" => Some("ς"),
        "tau" => Some("τ"),
        "upsilon" => Some("υ"),
        "phi" => Some("φ"),
        "varphi" => Some("ϕ"),
        "chi" => Some("χ"),
        "psi" => Some("ψ"),
        "omega" => Some("ω"),
        "Gamma" => Some("Γ"),
        "Delta" => Some("Δ"),
        "Theta" => Some("Θ"),
        "Lambda" => Some("Λ"),
        "Xi" => Some("Ξ"),
        "Pi" => Some("Π"),
        "Sigma" => Some("Σ"),
        "Upsilon" => Some("Υ"),
        "Phi" => Some("Φ"),
        "Psi" => Some("Ψ"),
        "Omega" => Some("Ω"),
        "sum" => Some("∑"),
        "prod" => Some("∏"),
        "coprod" => Some("∐"),
        "int" => Some("∫"),
        "iint" => Some("∬"),
        "iiint" => Some("∭"),
        "oint" => Some("∮"),
        "partial" => Some("∂"),
        "nabla" => Some("∇"),
        "infty" => Some("∞"),
        "forall" => Some("∀"),
        "exists" => Some("∃"),
        "nexists" => Some("∄"),
        "emptyset" => Some("∅"),
        "varnothing" => Some("∅"),
        "in" => Some("∈"),
        "notin" => Some("∉"),
        "ni" => Some("∋"),
        "subset" => Some("⊂"),
        "supset" => Some("⊃"),
        "subseteq" => Some("⊆"),
        "supseteq" => Some("⊇"),
        "cup" => Some("∪"),
        "cap" => Some("∩"),
        "setminus" => Some("∖"),
        "vee" => Some("∨"),
        "wedge" => Some("∧"),
        "oplus" => Some("⊕"),
        "otimes" => Some("⊗"),
        "odot" => Some("⊙"),
        "times" => Some("×"),
        "div" => Some("÷"),
        "pm" => Some("±"),
        "mp" => Some("∓"),
        "cdot" => Some("·"),
        "star" => Some("⋆"),
        "circ" => Some("∘"),
        "bullet" => Some("•"),
        "leq" | "le" => Some("≤"),
        "geq" | "ge" => Some("≥"),
        "neq" | "ne" => Some("≠"),
        "approx" => Some("≈"),
        "equiv" => Some("≡"),
        "sim" => Some("∼"),
        "simeq" => Some("≃"),
        "cong" => Some("≅"),
        "propto" => Some("∝"),
        "ll" => Some("≪"),
        "gg" => Some("≫"),
        "prec" => Some("≺"),
        "succ" => Some("≻"),
        "parallel" => Some("∥"),
        "perp" => Some("⊥"),
        "angle" => Some("∠"),
        "triangle" => Some("△"),
        "rightarrow" | "to" => Some("→"),
        "leftarrow" | "gets" => Some("←"),
        "leftrightarrow" => Some("↔"),
        "Rightarrow" | "implies" => Some("⇒"),
        "Leftarrow" | "impliedby" => Some("⇐"),
        "Leftrightarrow" | "iff" => Some("⇔"),
        "uparrow" => Some("↑"),
        "downarrow" => Some("↓"),
        "mapsto" => Some("↦"),
        "ldots" | "dotsc" | "dotso" => Some("…"),
        "cdots" | "dotsb" | "dotsm" | "dotsi" => Some("⋯"),
        "vdots" => Some("⋮"),
        "ddots" => Some("⋱"),
        "quad" => Some("  "),
        "ell" => Some("ℓ"),
        "hbar" => Some("ℏ"),
        "Re" => Some("ℜ"),
        "Im" => Some("ℑ"),
        "aleph" => Some("ℵ"),
        "dag" | "dagger" => Some("†"),
        "ddag" => Some("‡"),
        "neg" | "lnot" => Some("¬"),
        "top" => Some("⊤"),
        "bot" => Some("⊥"),
        "vdash" => Some("⊢"),
        "models" => Some("⊨"),
        "therefore" => Some("∴"),
        "because" => Some("∵"),
        "surd" => Some("√"),
        "langle" => Some("⟨"),
        "rangle" => Some("⟩"),
        "lceil" => Some("⌈"),
        "rceil" => Some("⌉"),
        "lfloor" => Some("⌊"),
        "rfloor" => Some("⌋"),
        "checkmark" => Some("✓"),
        "degree" => Some("°"),
        "colon" => Some(":"),
        "sin" => Some("sin"),
        "cos" => Some("cos"),
        "tan" => Some("tan"),
        "cot" => Some("cot"),
        "sec" => Some("sec"),
        "csc" => Some("csc"),
        "arcsin" => Some("arcsin"),
        "arccos" => Some("arccos"),
        "arctan" => Some("arctan"),
        "sinh" => Some("sinh"),
        "cosh" => Some("cosh"),
        "tanh" => Some("tanh"),
        "log" => Some("log"),
        "ln" => Some("ln"),
        "lg" => Some("lg"),
        "exp" => Some("exp"),
        "det" => Some("det"),
        "dim" => Some("dim"),
        "ker" => Some("ker"),
        "deg" => Some("deg"),
        "gcd" => Some("gcd"),
        "max" => Some("max"),
        "min" => Some("min"),
        "sup" => Some("sup"),
        "inf" => Some("inf"),
        "lim" => Some("lim"),
        "arg" => Some("arg"),
        "Pr" => Some("Pr"),
        "hom" => Some("hom"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_text() {
        let tree = parse_math("hello");
        assert_eq!(tree.nodes.len(), 1);
        assert_eq!(tree.nodes[0], MathNode::Text("hello".to_string()));
    }

    #[test]
    fn parses_greek_letter() {
        let tree = parse_math("\\alpha");
        assert_eq!(tree.nodes.len(), 1);
        assert_eq!(tree.nodes[0], MathNode::Symbol("α".to_string()));
    }

    #[test]
    fn parses_superscript() {
        let tree = parse_math("x^2");
        assert_eq!(tree.nodes.len(), 1);
        match &tree.nodes[0] {
            MathNode::Superscript { base, sup } => {
                assert_eq!(**base, MathNode::Text("x".to_string()));
                assert_eq!(**sup, MathNode::Text("2".to_string()));
            }
            _ => panic!("expected superscript"),
        }
    }

    #[test]
    fn parses_subscript() {
        let tree = parse_math("x_i");
        assert_eq!(tree.nodes.len(), 1);
        match &tree.nodes[0] {
            MathNode::Subscript { base, sub } => {
                assert_eq!(**base, MathNode::Text("x".to_string()));
                assert_eq!(**sub, MathNode::Text("i".to_string()));
            }
            _ => panic!("expected subscript"),
        }
    }

    #[test]
    fn parses_fraction() {
        let tree = parse_math("\\frac{a}{b}");
        assert_eq!(tree.nodes.len(), 1);
        match &tree.nodes[0] {
            MathNode::Fraction { numerator, denominator } => {
                assert_eq!(**numerator, MathNode::Text("a".to_string()));
                assert_eq!(**denominator, MathNode::Text("b".to_string()));
            }
            _ => panic!("expected fraction"),
        }
    }

    #[test]
    fn parses_sqrt() {
        let tree = parse_math("\\sqrt{x}");
        match &tree.nodes[0] {
            MathNode::Sqrt { content, index } => {
                assert_eq!(**content, MathNode::Text("x".to_string()));
                assert!(index.is_none());
            }
            _ => panic!("expected sqrt"),
        }
    }

    #[test]
    fn parses_nth_root() {
        let tree = parse_math("\\sqrt[3]{x}");
        match &tree.nodes[0] {
            MathNode::Sqrt { content, index } => {
                assert_eq!(**content, MathNode::Text("x".to_string()));
                assert!(index.is_some());
            }
            _ => panic!("expected sqrt"),
        }
    }

    #[test]
    fn display_text_for_fraction() {
        let tree = parse_math("\\frac{a}{b}");
        assert_eq!(math_tree_to_display_text(&tree), "a/b");
    }

    #[test]
    fn display_text_for_sqrt() {
        let tree = parse_math("\\sqrt{x}");
        assert_eq!(math_tree_to_display_text(&tree), "√(x)");
    }

    #[test]
    fn display_text_for_greek() {
        let tree = parse_math("\\alpha + \\beta");
        let text = math_tree_to_display_text(&tree);
        assert!(text.contains("α"));
        assert!(text.contains("β"));
    }

    #[test]
    fn display_text_uses_unicode_superscript() {
        let tree = parse_math("x^2");
        assert_eq!(math_tree_to_display_text(&tree), "x²");
    }

    #[test]
    fn display_text_uses_unicode_subscript() {
        let tree = parse_math("x_i");
        let text = math_tree_to_display_text(&tree);
        assert!(text.contains("ᵢ"));
    }

    #[test]
    fn einstein_formula() {
        let tree = parse_math("E = mc^2");
        assert_eq!(math_tree_to_display_text(&tree), "E = mc²");
    }

    #[test]
    fn pythagorean_theorem() {
        let tree = parse_math("a^2 + b^2 = c^2");
        let text = math_tree_to_display_text(&tree);
        assert!(text.contains("²"));
    }

    #[test]
    fn quadratic_formula() {
        let tree = parse_math("\\frac{-b \\pm \\sqrt{b^2 - 4ac}}{2a}");
        let text = math_tree_to_display_text(&tree);
        assert!(text.contains("±"));
        assert!(text.contains("√"));
    }

    #[test]
    fn escaped_braces() {
        let tree = parse_math("\\{x\\}");
        assert_eq!(math_tree_to_display_text(&tree), "{x}");
    }

    #[test]
    fn empty_source() {
        let tree = parse_math("");
        assert!(tree.nodes.is_empty());
    }

    #[test]
    fn display_text_for_overline() {
        let tree = parse_math("\\overline{AB}");
        let text = math_tree_to_display_text(&tree);
        assert!(text.contains("AB"));
    }

    #[test]
    fn unknown_command_fallback() {
        let tree = parse_math("\\unknowncmd");
        let text = math_tree_to_display_text(&tree);
        assert_eq!(text, "\\unknowncmd");
    }

    #[test]
    fn multiple_expressions() {
        let tree = parse_math("a + b = c");
        let text = math_tree_to_display_text(&tree);
        assert_eq!(text, "a + b = c");
    }

    #[test]
    fn nested_superscript_in_fraction() {
        let tree = parse_math("\\frac{x^2}{y^3}");
        let text = math_tree_to_display_text(&tree);
        assert!(text.contains("²"));
        assert!(text.contains("³"));
    }

    #[test]
    fn integral_expression() {
        let tree = parse_math("\\int_0^1 f(x) dx");
        let text = math_tree_to_display_text(&tree);
        assert!(text.contains("∫"));
    }

    #[test]
    fn infinity_and_comparison() {
        let tree = parse_math("\\infty \\leq \\pi");
        let text = math_tree_to_display_text(&tree);
        assert!(text.contains("∞"));
        assert!(text.contains("≤"));
        assert!(text.contains("π"));
    }

    #[test]
    fn cache_returns_same_result() {
        let mut cache = MathRenderCache::new();
        let t1 = cache.get_or_parse("x^2", MathRenderMode::Display, 17.0);
        let t2 = cache.get_or_parse("x^2", MathRenderMode::Display, 17.0);
        assert_eq!(t1, t2);
    }

    #[test]
    fn cache_different_source() {
        let mut cache = MathRenderCache::new();
        let t1 = cache.get_or_parse("x^2", MathRenderMode::Display, 17.0);
        let t2 = cache.get_or_parse("y^2", MathRenderMode::Display, 17.0);
        assert_ne!(t1, t2);
    }

    #[test]
    fn sum_with_limits() {
        let tree = parse_math("\\sum_{i=1}^{n} x_i");
        let text = math_tree_to_display_text(&tree);
        assert!(text.contains("∑"));
    }

    #[test]
    fn highlight_frac() {
        let tokens = highlight_math_source("\\frac{a}{b}");
        assert_eq!(tokens.len(), 6);
        assert_eq!(tokens[0].token_type, MathTokenType::Command);
        assert_eq!(tokens[0].text, "\\frac");
    }

    #[test]
    fn highlight_numbers() {
        let tokens = highlight_math_source("x = 42");
        let numbers: Vec<_> = tokens
            .iter()
            .filter(|t| t.token_type == MathTokenType::Number)
            .collect();
        assert_eq!(numbers.len(), 1);
        assert_eq!(numbers[0].text, "42");
    }

    #[test]
    fn highlight_empty() {
        let tokens = highlight_math_source("");
        assert!(tokens.is_empty());
    }
}
