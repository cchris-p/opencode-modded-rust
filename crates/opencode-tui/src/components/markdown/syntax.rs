use once_cell::sync::Lazy;
use ratatui::{
    style::{Modifier, Style},
    text::Span,
};
use syntect::{
    parsing::{ParseState, ScopeStack, SyntaxReference, SyntaxSet},
    util::LinesWithEndings,
};

use super::code_block::CodeTheme;

static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);

pub fn highlight_code(
    code: &str,
    language: &str,
    code_theme: &CodeTheme,
) -> Option<Vec<Vec<Span<'static>>>> {
    let syntax = find_syntax(language)?;
    let mut parse_state = ParseState::new(syntax);
    let mut scope_stack = ScopeStack::new();

    let mut output = Vec::new();
    for line in LinesWithEndings::from(code) {
        let ops = parse_state.parse_line(line, &SYNTAX_SET).ok()?;
        let mut spans: Vec<Span<'static>> = Vec::new();
        let mut cursor = 0usize;

        for (offset, op) in ops {
            if offset > cursor {
                push_segment(&mut spans, &line[cursor..offset], &scope_stack, code_theme);
                cursor = offset;
            }
            scope_stack.apply(&op).ok()?;
        }
        if cursor < line.len() {
            push_segment(&mut spans, &line[cursor..], &scope_stack, code_theme);
        }

        if spans.is_empty() {
            spans.push(Span::styled(
                String::new(),
                Style::default().fg(code_theme.text),
            ));
        }
        output.push(spans);
    }

    Some(output)
}

fn push_segment(
    spans: &mut Vec<Span<'static>>,
    segment: &str,
    scope_stack: &ScopeStack,
    code_theme: &CodeTheme,
) {
    let content = strip_line_endings(segment);
    if content.is_empty() {
        return;
    }
    let scope = scope_stack
        .as_slice()
        .last()
        .map(|scope| scope.build_string())
        .unwrap_or_default();
    spans.push(Span::styled(
        content.to_string(),
        scope_style(&scope, code_theme),
    ));
}

/// Maps a syntect scope name to an app theme token so code blocks follow the
/// active preset instead of a bundled syntect palette.
fn scope_style(scope: &str, theme: &CodeTheme) -> Style {
    if scope.contains("comment") {
        Style::default()
            .fg(theme.comment)
            .add_modifier(Modifier::ITALIC)
    } else if scope.contains("string") {
        Style::default().fg(theme.string)
    } else if scope.contains("constant.numeric")
        || scope.contains("constant.language")
        || scope.contains("constant.character")
        || scope.contains("number")
    {
        Style::default().fg(theme.number)
    } else if scope.contains("keyword") || scope.contains("storage") {
        Style::default()
            .fg(theme.keyword)
            .add_modifier(Modifier::BOLD)
    } else if scope.contains("entity.name.function")
        || scope.contains("support.function")
        || scope.contains("variable.function")
        || scope.contains("meta.function-call")
    {
        Style::default().fg(theme.function)
    } else if scope.contains("punctuation") || scope.contains("operator") {
        Style::default().fg(theme.punctuation)
    } else {
        Style::default().fg(theme.text)
    }
}

fn find_syntax(language: &str) -> Option<&'static SyntaxReference> {
    let trimmed = language.trim();
    if trimmed.is_empty() {
        return None;
    }

    let token = normalize_language_token(trimmed);
    SYNTAX_SET
        .find_syntax_by_token(&token)
        .or_else(|| SYNTAX_SET.find_syntax_by_extension(&token))
        .or_else(|| SYNTAX_SET.find_syntax_by_name(trimmed))
}

fn normalize_language_token(language: &str) -> String {
    match language.trim().to_ascii_lowercase().as_str() {
        "rs" => "rust".to_string(),
        "py" => "python".to_string(),
        "js" => "javascript".to_string(),
        "ts" => "typescript".to_string(),
        "yml" => "yaml".to_string(),
        "sh" | "shell" | "zsh" => "bash".to_string(),
        other => other.to_string(),
    }
}

fn strip_line_endings(segment: &str) -> &str {
    let no_newline = segment.strip_suffix('\n').unwrap_or(segment);
    no_newline.strip_suffix('\r').unwrap_or(no_newline)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn highlight_code_returns_none_for_unknown_language() {
        let theme = CodeTheme::default();
        let output = highlight_code("some text", "unknown_language_token_xyz", &theme);
        assert!(output.is_none());
    }

    #[test]
    fn highlight_code_rust_returns_spans() {
        let theme = CodeTheme::default();
        let output =
            highlight_code("fn main() {}\n", "rust", &theme).expect("rust syntax should exist");
        assert!(!output.is_empty());
        assert!(output.iter().any(|line| !line.is_empty()));
    }

    #[test]
    fn highlight_code_derives_colors_from_code_theme() {
        let mut theme = CodeTheme::default();
        theme.text = Color::Rgb(1, 1, 1);
        theme.keyword = Color::Rgb(2, 2, 2);
        theme.string = Color::Rgb(3, 3, 3);
        theme.number = Color::Rgb(4, 4, 4);
        theme.comment = Color::Rgb(5, 5, 5);
        theme.punctuation = Color::Rgb(6, 6, 6);
        theme.function = Color::Rgb(7, 7, 7);

        let tokens = [
            theme.text,
            theme.keyword,
            theme.string,
            theme.number,
            theme.comment,
            theme.punctuation,
            theme.function,
        ];

        let output = highlight_code("fn main() { let s = \"hi\"; }\n", "rust", &theme)
            .expect("rust syntax should exist");
        for span in output.iter().flatten() {
            let fg = span.style.fg.expect("each span carries a foreground color");
            assert!(
                tokens.contains(&fg),
                "span color {fg:?} is not an app theme token"
            );
        }
    }
}
