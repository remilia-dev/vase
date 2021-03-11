// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum StringEncoding {
    Default,
    U8,
    WChar16,
    WChar32,
    U16,
    U32,
}

impl StringEncoding {
    pub fn prefix(self) -> Option<&'static str> {
        match self {
            Self::Default => None,
            Self::U8 => Some("u8"),
            Self::WChar16 => Some("L"),
            Self::WChar32 => Some("L"),
            Self::U16 => Some("u"),
            Self::U32 => Some("U"),
        }
    }
}
