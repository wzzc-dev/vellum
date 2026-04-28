use std::collections::HashMap;

use tree_sitter::{Parser, TreeCursor};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeTokenType {
    Keyword,
    Function,
    String,
    Number,
    Comment,
    Type,
    Constant,
    Variable,
    Operator,
    Punctuation,
    Property,
    Tag,
    Attribute,
    Escape,
    Default,
}

#[derive(Debug, Clone)]
pub struct CodeHighlightSpan {
    pub start: usize,
    pub end: usize,
    pub token_type: CodeTokenType,
}

#[derive(Debug, Clone)]
pub struct CodeHighlightResult {
    pub spans: Vec<CodeHighlightSpan>,
}

pub struct CodeHighlighter {
    languages: HashMap<String, tree_sitter::Language>,
}

impl CodeHighlighter {
    pub fn new() -> Self {
        let mut languages = HashMap::new();

        let registrations: Vec<(&str, tree_sitter::Language)> = vec![
            ("rust", tree_sitter_rust::LANGUAGE.into()),
            ("javascript", tree_sitter_javascript::LANGUAGE.into()),
            ("js", tree_sitter_javascript::LANGUAGE.into()),
            ("python", tree_sitter_python::LANGUAGE.into()),
            ("py", tree_sitter_python::LANGUAGE.into()),
            ("go", tree_sitter_go::LANGUAGE.into()),
            ("golang", tree_sitter_go::LANGUAGE.into()),
            ("java", tree_sitter_java::LANGUAGE.into()),
            ("c", tree_sitter_c::LANGUAGE.into()),
            ("css", tree_sitter_css::LANGUAGE.into()),
            ("json", tree_sitter_json::LANGUAGE.into()),
            ("html", tree_sitter_html::LANGUAGE.into()),
        ];

        for (name, lang) in registrations {
            languages.insert(name.to_string(), lang);
        }

        Self { languages }
    }

    pub fn highlight(&self, language: &str, code: &str) -> Option<CodeHighlightResult> {
        let lang = self.languages.get(language)?;
        let mut parser = Parser::new();
        parser.set_language(lang).ok()?;

        let tree = parser.parse(code, None)?;
        let root = tree.root_node();

        let mut spans = Vec::new();
        let mut cursor = root.walk();
        Self::walk_tree(&mut cursor, &mut spans);

        if spans.is_empty() {
            spans.push(CodeHighlightSpan {
                start: 0,
                end: code.len(),
                token_type: CodeTokenType::Default,
            });
        } else {
            Self::fill_gaps(&mut spans, code.len());
        }

        Some(CodeHighlightResult { spans })
    }

    fn walk_tree(cursor: &mut TreeCursor, spans: &mut Vec<CodeHighlightSpan>) {
        loop {
            let node = cursor.node();
            let token_type = Self::classify_node(node.kind());

            if token_type != CodeTokenType::Default && node.child_count() == 0 {
                spans.push(CodeHighlightSpan {
                    start: node.start_byte(),
                    end: node.end_byte(),
                    token_type,
                });
            }

            if cursor.goto_first_child() {
                loop {
                    Self::walk_tree(cursor, spans);
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
                cursor.goto_parent();
            }

            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    fn classify_node(kind: &str) -> CodeTokenType {
        match kind {
            "function" | "function_definition" | "function_item" | "function_declaration"
            | "method_definition" | "method_declaration" | "arrow_function"
            | "generator_function" | "generator_function_declaration" => CodeTokenType::Function,

            "call_expression" | "method_call" => CodeTokenType::Function,

            "string" | "string_literal" | "string_content" | "raw_string_literal"
            | "interpreted_string_literal" | "template_string" | "template_substitution"
            | "char_literal" | "escape_sequence" => CodeTokenType::String,

            "number" | "integer_literal" | "float_literal" | "number_literal"
            | "decimal_number_literal" | "hex_integer_literal" | "octal_integer_literal"
            | "binary_integer_literal" | "big_integer_literal" => CodeTokenType::Number,

            "comment" | "line_comment" | "block_comment" | "doc_comment"
            | "html_comment" | "multiline_comment" | "singleline_comment" => CodeTokenType::Comment,

            "type_identifier" | "type" | "primitive_type" | "struct_item" | "enum_item"
            | "trait_item" | "impl_item" | "class_declaration" | "interface_declaration"
            | "type_alias_declaration" | "struct_expression" | "enum_expression"
            | "type_argument" | "generic_type" | "array_type" | "pointer_type"
            | "reference_type" | "tuple_type" | "unit_type" | "never_type"
            | "function_type" | "union_type" | "intersection_type" | "optional_type"
            | "conditional_type" | "parenthesized_type" | "object_type" => CodeTokenType::Type,

            "true" | "false" | "None" | "nil" | "null" | "undefined" | "NaN"
            | "Infinity" | "Some" | "Ok" | "Err" => CodeTokenType::Constant,

            "identifier" | "field_identifier" | "shorthand_property_identifier"
            | "property_identifier" | "variable_identifier" | "self" | "this" | "super"
            | "value_identifier" | "property_name" | "_" => CodeTokenType::Variable,

            "if" | "else" | "elif" | "for" | "while" | "loop" | "match" | "switch"
            | "case" | "default" | "break" | "continue" | "return" | "yield" | "await"
            | "async" | "try" | "catch" | "finally" | "throw" | "raise" | "except"
            | "fn" | "func" | "def" | "fun" | "let" | "const" | "var" | "val"
            | "mut" | "pub" | "private" | "protected" | "internal" | "static" | "extern"
            | "impl" | "trait" | "struct" | "enum" | "class" | "interface" | "extends"
            | "implements" | "where" | "use" | "import" | "export" | "from" | "as" | "in"
            | "ref" | "move" | "new" | "delete" | "sizeof" | "typeof"
            | "instanceof" | "void" | "do" | "goto" | "package" | "module" | "mod"
            | "crate" | "abstract" | "final" | "override" | "virtual"
            | "synchronized" | "volatile" | "native" | "transient" | "strictfp"
            | "throws" | "with" | "using" | "namespace" | "include" | "require"
            | "defer" | "go" | "select" | "range" | "chan" | "map" | "fallthrough"
            | "pass" | "lambda" | "global" | "nonlocal" | "assert" | "del" | "and"
            | "or" | "not" | "is" => CodeTokenType::Keyword,

            "&&" | "||" | "!" | "!=" | "==" | "<=" | ">=" | "<" | ">" | "+"
            | "-" | "*" | "/" | "%" | "&" | "|" | "^" | "~" | "<<" | ">>"
            | "=" | "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|=" | "^="
            | "<<=" | ">>=" | "&&=" | "||=" | "??" | "?."
            | "++" | "--" | "->" | "=>" | ".." | "..=" | "..." | "::" => CodeTokenType::Operator,

            "(" | ")" | "[" | "]" | "{" | "}" | "," | ";" | ":" | "." | "?"
            | "@" | "#" | "$" | "\\" | "`" => CodeTokenType::Punctuation,

            "pair" | "property_assignment" | "field_declaration" => CodeTokenType::Property,

            "jsx_opening_element" | "jsx_closing_element" | "jsx_self_closing_element"
            | "element" | "start_tag" | "end_tag" | "self_closing_tag" => CodeTokenType::Tag,

            "attribute" | "attribute_name" | "attribute_value" | "jsx_attribute"
            | "id_attribute" | "class_attribute" => CodeTokenType::Attribute,

            "char_escape_sequence" => CodeTokenType::Escape,

            _ => CodeTokenType::Default,
        }
    }

    pub fn fill_gaps(spans: &mut Vec<CodeHighlightSpan>, total_len: usize) {
        if spans.is_empty() {
            return;
        }

        spans.sort_by_key(|s| s.start);

        let mut filled = Vec::new();
        let mut pos = 0;

        for span in spans.iter() {
            if span.start > pos {
                filled.push(CodeHighlightSpan {
                    start: pos,
                    end: span.start,
                    token_type: CodeTokenType::Default,
                });
            }
            if span.start < pos {
                continue;
            }
            filled.push(span.clone());
            pos = span.end;
        }

        if pos < total_len {
            filled.push(CodeHighlightSpan {
                start: pos,
                end: total_len,
                token_type: CodeTokenType::Default,
            });
        }

        *spans = filled;
    }
}

impl Default for CodeHighlighter {
    fn default() -> Self {
        Self::new()
    }
}
