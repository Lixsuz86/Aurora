//! Rich compiler diagnostic messages, labels, and source snippet renderers.

use aurora_ast::{SourceFile, Span};
use std::fmt;

/// Severity level of a diagnostic message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticLevel {
    Help,
    Note,
    Warning,
    Error,
}

impl fmt::Display for DiagnosticLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
            Self::Warning => write!(f, "warning"),
            Self::Note => write!(f, "note"),
            Self::Help => write!(f, "help"),
        }
    }
}

/// A compiler diagnostic message tied to source positions with hints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub message: String,
    pub span: Span,
    pub label: Option<String>,
    pub hints: Vec<String>,
}

impl Diagnostic {
    /// Creates a new error diagnostic.
    pub fn error(message: impl Into<String>, span: Span) -> Self {
        Self {
            level: DiagnosticLevel::Error,
            message: message.into(),
            span,
            label: None,
            hints: Vec::new(),
        }
    }

    /// Creates a new warning diagnostic.
    pub fn warning(message: impl Into<String>, span: Span) -> Self {
        Self {
            level: DiagnosticLevel::Warning,
            message: message.into(),
            span,
            label: None,
            hints: Vec::new(),
        }
    }

    /// Adds an inline label pointing to the span.
    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Adds an actionable hint or suggestion.
    #[must_use]
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hints.push(hint.into());
        self
    }

    /// Renders a diagnostic into a human-readable string with source line snippets and arrows.
    #[must_use]
    pub fn render(&self, file: &SourceFile) -> String {
        use std::fmt::Write;
        let loc = file.location(self.span.start);
        let mut out = String::new();

        // Header: level: message
        let _ = writeln!(out, "{}: {}", self.level, self.message);

        // Location: --> filename:line:col
        let file_path = file
            .path
            .as_ref()
            .map_or_else(|| file.name.clone(), |p| p.display().to_string());
        let _ = writeln!(out, "  --> {}:{}:{}", file_path, loc.line, loc.column);

        // Line snippet if available
        if let Some(line_str) = file.get_line(loc.line) {
            let line_num_str = loc.line.to_string();
            let padding = " ".repeat(line_num_str.len());

            let _ = writeln!(out, "   {padding} |");
            let _ = writeln!(out, "{line_num_str} | {line_str}");

            // Underline highlight: ^^^
            let col_idx = loc.column.saturating_sub(1);
            let highlight_len = self.span.len().max(1);
            let carets = "^".repeat(highlight_len);
            let caret_padding = " ".repeat(col_idx);

            if let Some(label) = &self.label {
                let _ = writeln!(out, "   {padding} | {caret_padding}{carets} {label}");
            } else {
                let _ = writeln!(out, "   {padding} | {caret_padding}{carets}");
            }
            let _ = writeln!(out, "   {padding} |");
        }

        // Hints
        for hint in &self.hints {
            let _ = writeln!(out, "  = help: {hint}");
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aurora_ast::FileId;

    #[test]
    fn test_diagnostic_render() {
        let code = "let val = 0xZZ;\nlet y = 10;";
        let file = SourceFile::new(FileId(0), "test.au", code, None);
        let diag = Diagnostic::error("invalid hexadecimal digit in literal", Span::new(10, 14))
            .with_label("unknown hex character")
            .with_hint("hexadecimal digits must be between 0-9 and a-f");

        let rendered = diag.render(&file);
        assert!(rendered.contains("error: invalid hexadecimal digit in literal"));
        assert!(rendered.contains("--> test.au:1:11"));
        assert!(rendered.contains("0xZZ"));
        assert!(rendered.contains("^^^^ unknown hex character"));
        assert!(rendered.contains("= help: hexadecimal digits must be between 0-9 and a-f"));
    }
}
