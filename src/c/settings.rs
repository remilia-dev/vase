use std::path::Path;

use crate::sync::Arc;

pub struct CCompileSettings {
    pub version: CLangVersion,
    pub system_includes: Vec<Box<Path>>,
    pub local_includes: Vec<Box<Path>>,
    pub source_files: Vec<Arc<Path>>,
}

impl CCompileSettings {}

impl Default for CCompileSettings {
    fn default() -> Self {
        let mut res = CCompileSettings {
            version: CLangVersion::C89,
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
pub enum CLangVersion {
    C89,
    C99,
    C11,
    C17,
    C23,
}
