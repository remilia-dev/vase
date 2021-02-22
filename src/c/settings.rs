// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::path::Path;

use crate::sync::Arc;

pub struct CompileSettings {
    pub version: LangVersion,
    pub system_includes: Vec<Box<Path>>,
    pub local_includes: Vec<Box<Path>>,
    pub source_files: Vec<Arc<Path>>,
}

impl CompileSettings {}

impl Default for CompileSettings {
    fn default() -> Self {
        let mut res = CompileSettings {
            version: LangVersion::C89,
            system_includes: Vec::new(),
            local_includes: Vec::new(),
            source_files: Vec::new(),
        };
        // TODO: Make include path generic for the OS.
        res.system_includes.push(Box::from(Path::new("/usr/local/include")));
        res.system_includes.push(Box::from(Path::new("/usr/include/")));
        // TODO: Add compiler-specific header location.
        // It's debatable whether we should go with GCC's headers or Clang's headers.
        // Both of them will require support for some `__builtin_` keywords.
        res
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum LangVersion {
    C89,
    C99,
    C11,
    C17,
    C23,
}
