// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use crate::{
    c::CompileSettings,
    util::variant_list,
};

#[variant_list]
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StringEnc {
    Default,
    U8,
    WChar16,
    WChar32,
    U16,
    U32,
}

impl StringEnc {
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

    pub fn should_add(self, settings: &CompileSettings) -> bool {
        match self {
            Self::WChar16 => settings.wchar_is_16_bytes,
            Self::WChar32 => !settings.wchar_is_16_bytes,
            _ => true,
        }
    }
}
