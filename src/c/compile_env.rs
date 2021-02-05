// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::collections::HashMap;
use std::path::Path;

use rayon::{
    ThreadPool,
    ThreadPoolBuilder,
};

use crate::{
    c::{
        CCompileSettings,
        CIncludeType,
        CLangVersion,
        CLexerError,
        CStringType,
        CTokenKind,
        CTokenStack,
    },
    sync::{
        Arc,
        OnceArray,
    },
    util::{
        CachedString,
        StringCache,
    },
};

pub struct CCompileEnv {
    settings: CCompileSettings,
    threads: Arc<ThreadPool>,
    cache: StringCache,
    cached_to_keywords: HashMap<CachedString, CTokenKind>,
    cached_to_preprocessor: HashMap<CachedString, CTokenKind>,
    cached_to_str_prefix: HashMap<CachedString, CStringType>,
    // OPTIMIZATION: Maybe OnceArray should operate on Arcs rather than boxes.
    file_id_to_tokens: OnceArray<Result<Arc<CTokenStack>, CLexerError>>,
}
impl CCompileEnv {
    pub fn new(settings: CCompileSettings) -> CCompileEnv {
        // OPTIMIZATION: May be able to improve the hashmaps by using a different hasher or hashmap.
        let mut env = CCompileEnv {
            settings,
            threads: Arc::new(ThreadPoolBuilder::new().build().unwrap()),
            cache: StringCache::new(),
            cached_to_keywords: HashMap::new(),
            cached_to_preprocessor: HashMap::new(),
            cached_to_str_prefix: HashMap::new(),
            file_id_to_tokens: OnceArray::default(),
        };
        update_cache_maps(&mut env);
        env
    }

    pub fn settings(&self) -> &CCompileSettings {
        &self.settings
    }
    pub fn threads(&self) -> &Arc<ThreadPool> {
        &self.threads
    }
    pub fn cache(&self) -> &StringCache {
        &self.cache
    }
    pub fn cached_to_keywords(&self) -> &HashMap<CachedString, CTokenKind> {
        &self.cached_to_keywords
    }
    pub fn cached_to_preprocessor(&self) -> &HashMap<CachedString, CTokenKind> {
        &self.cached_to_preprocessor
    }
    pub fn cached_to_str_prefix(&self) -> &HashMap<CachedString, CStringType> {
        &self.cached_to_str_prefix
    }
    pub fn file_id_to_tokens(&self) -> &OnceArray<Result<Arc<CTokenStack>, CLexerError>> {
        &self.file_id_to_tokens
    }

    pub fn find_include(
        &self,
        inc_type: CIncludeType,
        filename: &CachedString,
        curr_file: Option<&Arc<Path>>,
    ) -> Option<Arc<Path>> {
        let get_child_file_if_exists = |dir: &Path| -> Option<Arc<Path>> {
            let path_buffer = dir.join(filename.string());
            return if path_buffer.exists() {
                Some(Arc::from(path_buffer.as_path()))
            } else {
                None
            };
        };

        if inc_type.check_relative() {
            if let Some(curr_file) = curr_file {
                if let Some(target) = curr_file
                    .parent()
                    .and_then(|curr_dir| get_child_file_if_exists(curr_dir))
                {
                    if !inc_type.ignore_own_file() || target != *curr_file {
                        return Some(target);
                    }
                }
            }

            for search_dir in &self.settings.local_includes {
                if let Some(target) = get_child_file_if_exists(search_dir) {
                    return Some(target);
                }
            }
        }

        for search_dir in &self.settings.system_includes {
            if let Some(target) = get_child_file_if_exists(search_dir) {
                return Some(target);
            }
        }

        None
    }
}

fn update_cache_maps(env: &mut CCompileEnv) {
    use CTokenKind::*;
    let version = env.settings.version;

    let mut map_keyword = |s: &str, kind: CTokenKind| {
        let cached = env.cache.get_or_cache(s);
        env.cached_to_keywords.insert(cached, kind);
    };
    map_keyword("auto", Auto);
    map_keyword("break", Break);
    map_keyword("case", Case);
    map_keyword("char", Char);
    map_keyword("const", Const);
    map_keyword("continue", Continue);
    map_keyword("default", Default);
    map_keyword("do", Do);
    map_keyword("double", Double);
    map_keyword("else", Else);
    map_keyword("enum", Enum);
    map_keyword("extern", Extern);
    map_keyword("float", Float);
    map_keyword("for", For);
    map_keyword("goto", Goto);
    map_keyword("if", If);
    map_keyword("int", Int);
    map_keyword("long", Long);
    map_keyword("register", Register);
    map_keyword("return", Return);
    map_keyword("short", Short);
    map_keyword("signed", Signed);
    map_keyword("sizeof", Sizeof);
    map_keyword("static", Static);
    map_keyword("struct", Struct);
    map_keyword("switch", Switch);
    map_keyword("typedef", Typedef);
    map_keyword("union", Union);
    map_keyword("unsigned", Unsigned);
    map_keyword("void", Void);
    map_keyword("volatile", Volatile);
    map_keyword("while", While);
    if version >= CLangVersion::C99 {
        map_keyword("inline", Inline);
        map_keyword("restrict", Restrict);
        map_keyword("_Bool", Bool);
        map_keyword("_Complex", Complex);
        map_keyword("_Imaginary", Imaginary);
        map_keyword("_Pragma", Pragma);
    }
    if version >= CLangVersion::C11 {
        map_keyword("_Alignas", Alignas);
        map_keyword("_Alignof", Alignof);
        map_keyword("_Atomic", Atomic);
        map_keyword("_Generic", Generic);
        map_keyword("_Noreturn", Noreturn);
        map_keyword("_Static_assert", StaticAssert);
        map_keyword("_Thread_local", ThreadLocal);
    }
    if version >= CLangVersion::C23 {
        map_keyword("_Decimal32", Decimal32);
        map_keyword("_Decimal64", Decimal64);
        map_keyword("_Decimal128", Decimal128);
    }

    let mut map_preprocessor = |s: &str, pre: CTokenKind| {
        let cached = env.cache.get_or_cache(s);
        env.cached_to_preprocessor.insert(cached, pre);
    };
    map_preprocessor("if", PreIf { link: usize::MAX });
    map_preprocessor("ifdef", PreIfDef { link: usize::MAX });
    map_preprocessor("ifndef", PreIfNDef { link: usize::MAX });
    map_preprocessor("elif", PreElif { link: usize::MAX });
    map_preprocessor("else", PreElse { link: usize::MAX });
    map_preprocessor("endif", PreEndIf);
    map_preprocessor("define", PreDefine);
    map_preprocessor("undef", PreUndef);
    map_preprocessor("line", PreLine);
    map_preprocessor("error", PreError);
    map_preprocessor("pragma", PrePragma);
    map_preprocessor("include", PreInclude);
    map_preprocessor("include_next", PreIncludeNext);
    map_preprocessor("warning", PreWarning);

    let mut map_str_prefix = |s: &str, str: CStringType| {
        let cached = env.cache.get_or_cache(s);
        env.cached_to_str_prefix.insert(cached, str);
    };
    map_str_prefix("u8", CStringType::U8);
    map_str_prefix("u", CStringType::U16);
    map_str_prefix("U", CStringType::U32);
    map_str_prefix("L", CStringType::WChar);
}
