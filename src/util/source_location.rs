// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::convert::TryInto;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceLocation {
    file_id: FileId,
    /// The byte in the file this location starts at.
    pub byte: u32,
    /// The number of bytes this source location represents.
    pub byte_length: u16,
}

impl SourceLocation {
    /// Creates a new source location that represents a specific range of bytes in a file.
    pub fn new(file_id: FileId, byte: u32, byte_length: u16) -> Self {
        SourceLocation { file_id, byte, byte_length }
    }
    /// Creates a new source location that represents the first byte in a file.
    pub fn new_first_byte(file_id: FileId) -> Self {
        SourceLocation { file_id, byte: 0, byte_length: 1 }
    }
    /// The id of the file this source location is in.
    pub fn file_id(&self) -> FileId {
        self.file_id
    }
    /// Returns the range of bytes this source location represents.
    ///
    /// The range returned is typed as a usize, but the full extend
    /// of the range isn't possible.
    pub fn range(&self) -> std::ops::Range<usize> {
        let byte = self.byte as usize;
        let byte_length = self.byte_length as usize;
        byte..(byte + byte_length)
    }
    /// Potentially returns a new source location that contains all
    /// of this source location and all of another source location.
    ///
    /// This function will return None if the source locations are from different files.
    pub fn through(&self, other: &SourceLocation) -> Option<SourceLocation> {
        if self.file_id == other.file_id {
            let start = self.byte.min(other.byte);
            let end = (self.byte + self.byte_length as u32) //
                .max(other.byte + other.byte_length as u32);
            let length = (end - start).try_into().unwrap_or(u16::MAX);
            Some(SourceLocation::new(self.file_id, start, length))
        } else {
            None
        }
    }
}
/// A type alias representing the numeric type that represents the id of a file.
/// # Warning
/// While this type is currently a u32, it may change in the future to another
/// numeric type or a structure type. However, this type should never be
/// larger than 4 bytes.
pub type FileId = u32;

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn range_matches_expected() {
        const START: usize = 23;
        const LENGTH: usize = 45;
        let test_case = SourceLocation::new(0, START as u32, LENGTH as u16);
        assert_eq!(test_case.range().start, START);
        assert_eq!(test_case.range().len(), LENGTH);
    }

    #[test]
    fn through_matches_expected() {
        let start = SourceLocation::new(0, 3, 4);
        let end = SourceLocation::new(0, 20, 3);
        assert_eq!(start.through(&end), Some(SourceLocation::new(0, 3, 20)));
    }

    #[test]
    fn through_reversed_matches_expected() {
        let start = SourceLocation::new(0, 20, 3);
        let end = SourceLocation::new(0, 3, 4);
        assert_eq!(start.through(&end), Some(SourceLocation::new(0, 3, 20)));
    }

    #[test]
    fn through_returns_none_when_different_files() {
        let start = SourceLocation::new(0, 0, 10);
        let end = SourceLocation::new(1, 10, 10);
        assert!(start.through(&end).is_none());
    }
}
