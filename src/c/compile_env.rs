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
        CPreprocessorType,
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
    cached_to_preprocessor: HashMap<CachedString, CPreprocessorType>,
    cached_to_str_prefix: HashMap<CachedString, CStringType>,
    file_id_to_tokens: OnceArray<Result<CTokenStack, CLexerError>>,
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
    pub fn cached_to_preprocessor(&self) -> &HashMap<CachedString, CPreprocessorType> {
        &self.cached_to_preprocessor
    }
    pub fn cached_to_str_prefix(&self) -> &HashMap<CachedString, CStringType> {
        &self.cached_to_str_prefix
    }
    pub fn file_id_to_tokens(&self) -> &OnceArray<Result<CTokenStack, CLexerError>> {
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
    let version = env.settings.version;

    let mut map_keyword = |s: &str, kind: CTokenKind| {
        let cached = env.cache.get_or_cache(s);
        env.cached_to_keywords.insert(cached, kind);
    };
    map_keyword("auto", CTokenKind::Auto);
    map_keyword("break", CTokenKind::Break);
    map_keyword("case", CTokenKind::Case);
    map_keyword("char", CTokenKind::Char);
    map_keyword("const", CTokenKind::Const);
    map_keyword("continue", CTokenKind::Continue);
    map_keyword("default", CTokenKind::Default);
    map_keyword("do", CTokenKind::Do);
    map_keyword("double", CTokenKind::Double);
    map_keyword("else", CTokenKind::Else);
    map_keyword("enum", CTokenKind::Enum);
    map_keyword("extern", CTokenKind::Extern);
    map_keyword("float", CTokenKind::Float);
    map_keyword("for", CTokenKind::For);
    map_keyword("goto", CTokenKind::Goto);
    map_keyword("if", CTokenKind::If);
    map_keyword("int", CTokenKind::Int);
    map_keyword("long", CTokenKind::Long);
    map_keyword("register", CTokenKind::Register);
    map_keyword("return", CTokenKind::Return);
    map_keyword("short", CTokenKind::Short);
    map_keyword("signed", CTokenKind::Signed);
    map_keyword("sizeof", CTokenKind::Sizeof);
    map_keyword("static", CTokenKind::Static);
    map_keyword("struct", CTokenKind::Struct);
    map_keyword("switch", CTokenKind::Switch);
    map_keyword("typedef", CTokenKind::Typedef);
    map_keyword("union", CTokenKind::Union);
    map_keyword("unsigned", CTokenKind::Unsigned);
    map_keyword("void", CTokenKind::Void);
    map_keyword("volatile", CTokenKind::Volatile);
    map_keyword("while", CTokenKind::While);
    if version >= CLangVersion::C99 {
        map_keyword("inline", CTokenKind::Inline);
        map_keyword("restrict", CTokenKind::Restrict);
        map_keyword("_Bool", CTokenKind::Bool);
        map_keyword("_Complex", CTokenKind::Complex);
        map_keyword("_Imaginary", CTokenKind::Imaginary);
        map_keyword("_Pragma", CTokenKind::Pragma);
    }
    if version >= CLangVersion::C11 {
        map_keyword("_Alignas", CTokenKind::Alignas);
        map_keyword("_Alignof", CTokenKind::Alignof);
        map_keyword("_Atomic", CTokenKind::Atomic);
        map_keyword("_Generic", CTokenKind::Generic);
        map_keyword("_Noreturn", CTokenKind::Noreturn);
        map_keyword("_Static_assert", CTokenKind::StaticAssert);
        map_keyword("_Thread_local", CTokenKind::ThreadLocal);
    }
    if version >= CLangVersion::C23 {
        map_keyword("_Decimal32", CTokenKind::Decimal32);
        map_keyword("_Decimal64", CTokenKind::Decimal64);
        map_keyword("_Decimal128", CTokenKind::Decimal128);
    }

    let mut map_preprocessor = |s: &str, pre: CPreprocessorType| {
        let cached = env.cache.get_or_cache(s);
        env.cached_to_preprocessor.insert(cached, pre);
    };
    map_preprocessor("if", CPreprocessorType::If { link: u32::MAX });
    map_preprocessor("ifdef", CPreprocessorType::IfDef { link: u32::MAX });
    map_preprocessor("ifndef", CPreprocessorType::IfNDef { link: u32::MAX });
    map_preprocessor("elif", CPreprocessorType::Elif { link: u32::MAX });
    map_preprocessor("else", CPreprocessorType::Else { link: u32::MAX });
    map_preprocessor("endif", CPreprocessorType::EndIf);
    map_preprocessor("define", CPreprocessorType::Define);
    map_preprocessor("undef", CPreprocessorType::Undef);
    map_preprocessor("line", CPreprocessorType::Line);
    map_preprocessor("error", CPreprocessorType::Error);
    map_preprocessor("pragma", CPreprocessorType::Pragma);
    map_preprocessor("include", CPreprocessorType::Include);
    map_preprocessor("include_next", CPreprocessorType::IncludeNext);
    map_preprocessor("warning", CPreprocessorType::Warning);

    let mut map_str_prefix = |s: &str, str: CStringType| {
        let cached = env.cache.get_or_cache(s);
        env.cached_to_str_prefix.insert(cached, str);
    };
    map_str_prefix("u8", CStringType::U8);
    map_str_prefix("u", CStringType::U16);
    map_str_prefix("U", CStringType::U32);
    map_str_prefix("L", CStringType::WCHAR);
}
