// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.

/// Represents a location in a file/string by its line and column.
///
/// Both line and column are u32. I think 4GBs worth of lines or columns is enough.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct SourceLocation {
    line: u32,
    column: u32,
}
impl SourceLocation {
    /// Creates a new source location at the given line and column.
    pub fn new(line: u32, column: u32) -> SourceLocation {
        SourceLocation { line, column }
    }
    /// Returns the line this location is at.
    pub fn line(&self) -> u32 {
        self.line
    }
    /// Returns the column this location is at.
    pub fn column(&self) -> u32 {
        self.column
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_uses_parameters_correctly() {
        let test_val = SourceLocation::new(1, 2);
        assert_eq!(test_val.line(), 1);
        assert_eq!(test_val.column(), 2);
    }

    #[test]
    fn can_copy() {
        let original = SourceLocation::new(1, 2);
        let copy = original;
        assert_eq!(original, copy);
    }
}
