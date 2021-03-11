// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum IncludeType {
    IncludeSystem, // For #include <file>
    IncludeLocal,  // For #include "file"
    IncludeNext,   // For #include_next "file"
}

impl IncludeType {
    pub fn is_end_char(self, c: char) -> bool {
        match c {
            '"' => self == IncludeType::IncludeLocal,
            '>' => self == IncludeType::IncludeSystem,
            _ => false,
        }
    }

    pub fn check_relative(self) -> bool {
        return matches!(self, IncludeType::IncludeLocal | IncludeType::IncludeNext);
    }

    pub fn ignore_own_file(self) -> bool {
        return matches!(self, IncludeType::IncludeNext);
    }
}

impl fmt::Display for IncludeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::IncludeSystem => write!(f, "system include"),
            Self::IncludeLocal => write!(f, "local/relative include"),
            Self::IncludeNext => write!(f, "#include_next include"),
        }
    }
}
