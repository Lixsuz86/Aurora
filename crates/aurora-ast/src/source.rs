//! Source file tracking and line-column location mapping.

use crate::span::{SourceLocation, Span};
use std::path::PathBuf;
use std::sync::Arc;

/// A unique identifier for a source file registered in a `SourceMap`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileId(pub u32);

/// Represents a loaded source file with its content and line start offsets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    pub id: FileId,
    pub name: String,
    pub path: Option<PathBuf>,
    pub source: Arc<str>,
    /// Byte offsets where each line begins in `source`.
    line_starts: Vec<usize>,
}

impl SourceFile {
    /// Creates a new `SourceFile` and pre-computes line start byte offsets.
    pub fn new(
        id: FileId,
        name: impl Into<String>,
        source: impl Into<String>,
        path: Option<PathBuf>,
    ) -> Self {
        let source_str: String = source.into();
        let line_starts = compute_line_starts(&source_str);
        Self {
            id,
            name: name.into(),
            path,
            source: Arc::from(source_str.into_boxed_str()),
            line_starts,
        }
    }

    /// Converts a byte offset into a 1-indexed (line, column) `SourceLocation`.
    #[must_use]
    pub fn location(&self, byte_offset: usize) -> SourceLocation {
        let offset = byte_offset.min(self.source.len());

        // Binary search for the line containing `offset`
        let line_idx = match self.line_starts.binary_search(&offset) {
            Ok(exact) => exact,
            Err(insertion_point) => insertion_point.saturating_sub(1),
        };

        let line_start_offset = self.line_starts[line_idx];
        let line_slice = &self.source[line_start_offset..offset];
        let column = line_slice.chars().count() + 1;
        let line = line_idx + 1;

        SourceLocation::new(line, column)
    }

    /// Returns the source text of the given 1-indexed line number (without trailing \r or \n).
    #[must_use]
    pub fn get_line(&self, line_number: usize) -> Option<&str> {
        if line_number == 0 || line_number > self.line_starts.len() {
            return None;
        }

        let line_idx = line_number - 1;
        let start = self.line_starts[line_idx];
        let end = if line_idx + 1 < self.line_starts.len() {
            self.line_starts[line_idx + 1]
        } else {
            self.source.len()
        };

        let raw = &self.source[start..end];
        Some(raw.trim_end_matches(['\r', '\n']))
    }

    /// Returns the total number of lines in this source file.
    #[must_use]
    pub fn total_lines(&self) -> usize {
        self.line_starts.len()
    }

    /// Slices text covered by a given Span.
    #[must_use]
    pub fn slice_span(&self, span: Span) -> &str {
        let start = span.start.min(self.source.len());
        let end = span.end.min(self.source.len()).max(start);
        &self.source[start..end]
    }
}

/// Registry storing multiple source files.
#[derive(Debug, Default, Clone)]
pub struct SourceMap {
    files: Vec<Arc<SourceFile>>,
}

impl SourceMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a source file string into the source map and returns its Arc reference.
    pub fn add_file(
        &mut self,
        name: impl Into<String>,
        source: impl Into<String>,
        path: Option<PathBuf>,
    ) -> Arc<SourceFile> {
        let id = FileId(self.files.len() as u32);
        let file = Arc::new(SourceFile::new(id, name, source, path));
        self.files.push(Arc::clone(&file));
        file
    }

    /// Retrieves a file by its `FileId`.
    #[must_use]
    pub fn get(&self, id: FileId) -> Option<&Arc<SourceFile>> {
        self.files.get(id.0 as usize)
    }
}

fn compute_line_starts(source: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (idx, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            starts.push(idx + 1);
        }
    }
    starts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_location_and_line_lookup() {
        let code = "fn main() {\n    let x = 42\n    return x\n}";
        let file = SourceFile::new(FileId(0), "test.au", code, None);

        // First line
        let loc1 = file.location(0);
        assert_eq!(loc1.line, 1);
        assert_eq!(loc1.column, 1);

        // Second line ("    let x") -> offset of 'l' is 12 + 4 = 16
        let loc2 = file.location(16);
        assert_eq!(loc2.line, 2);
        assert_eq!(loc2.column, 5);
        assert_eq!(file.get_line(2), Some("    let x = 42"));
    }
}
