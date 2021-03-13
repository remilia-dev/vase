// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::path::Path;

use crate::sync::Arc;

pub struct CompileSettings {
    pub version: LangVersion,
    pub system_includes: Vec<Box<Path>>,
    pub local_includes: Vec<Box<Path>>,
    pub source_files: Vec<Arc<Path>>,
    pub wchar_is_16_bytes: bool,
}

impl CompileSettings {}

impl Default for CompileSettings {
    fn default() -> Self {
        let mut res = CompileSettings {
            version: LangVersion::C89,
            system_includes: Vec::new(),
            local_includes: Vec::new(),
            source_files: Vec::new(),
            wchar_is_16_bytes: false,
        };
        let mut current_exe = std::env::current_exe().unwrap();
        current_exe.pop();
        current_exe.push("include");

        // TODO: Make include path generic for the OS.
        res.system_includes.push(Box::from(current_exe.as_path()));
        res.system_includes.push(Box::from(Path::new("/usr/local/include")));
        res.system_includes.push(Box::from(Path::new("/usr/include/")));
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
