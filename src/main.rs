// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::path::Path;

use vase::c::*;
use vase::sync::Arc;

fn main() {
    let mut settings = CCompileSettings::default();
    settings.source_files.push(Arc::from(Path::new("./test.c")));
    let env = Arc::new(CCompileEnv::new(settings));
    let mut lexer = CMultiLexer::new(env.clone());
    lexer.lex_multi_threaded(&*env.settings().source_files);
}
