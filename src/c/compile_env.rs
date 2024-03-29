// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::{
    collections::HashMap,
    path::Path,
};

#[cfg(feature = "multithreading")]
use rayon::{
    ThreadPool,
    ThreadPoolBuilder,
};

use crate::{
    c::{
        CompileSettings,
        FileTokens,
        IncludeType,
        Keyword,
        StringEnc,
        TokenKind,
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

pub struct CompileEnv {
    settings: CompileSettings,
    #[cfg(feature = "multithreading")]
    threads: Arc<ThreadPool>,
    cache: StringCache,
    // OPTIMIZATION: A two-way map may be better than two separate maps.
    cached_to_keywords: HashMap<CachedString, Keyword>,
    keyword_to_cached: HashMap<Keyword, CachedString>,
    cached_to_preprocessor: HashMap<CachedString, TokenKind>,
    cached_to_str_prefix: HashMap<CachedString, StringEnc>,
    pub file_id_to_tokens: OnceArray<FileTokens>,
}
impl CompileEnv {
    pub fn new(settings: CompileSettings) -> CompileEnv {
        // OPTIMIZATION: May be able to improve the hashmaps by using a different hasher or hashmap.
        let mut env = CompileEnv {
            settings,
            #[cfg(feature = "multithreading")]
            threads: Arc::new(ThreadPoolBuilder::new().build().unwrap()),
            cache: StringCache::new(),
            cached_to_keywords: HashMap::new(),
            keyword_to_cached: HashMap::new(),
            cached_to_preprocessor: HashMap::new(),
            cached_to_str_prefix: HashMap::new(),
            file_id_to_tokens: OnceArray::default(),
        };
        update_cache_maps(&mut env);
        env
    }

    pub fn settings(&self) -> &CompileSettings {
        &self.settings
    }

    #[cfg(feature = "multithreading")]
    pub fn threads(&self) -> &Arc<ThreadPool> {
        &self.threads
    }

    pub fn cache(&self) -> &StringCache {
        &self.cache
    }

    pub fn get_keyword(&self, v: &CachedString) -> Option<Keyword> {
        self.cached_to_keywords.get(v).cloned()
    }

    pub fn get_keyword_string(&self, v: Keyword) -> Option<&CachedString> {
        self.keyword_to_cached.get(&v)
    }

    pub fn get_definable_id<'a>(&'a self, v: &'a TokenKind) -> &'a CachedString {
        match *v {
            TokenKind::Identifier(ref id) => id,
            TokenKind::Keyword(keyword) => &self.keyword_to_cached[&keyword],
            _ => panic!(
                "Non-definable token does not have a definable id: {:?}",
                v
            ),
        }
    }

    pub fn get_preprocessor(&self, v: &CachedString) -> Option<TokenKind> {
        self.cached_to_preprocessor.get(v).cloned()
    }

    pub fn get_string_prefix(&self, v: &CachedString) -> Option<StringEnc> {
        self.cached_to_str_prefix.get(v).cloned()
    }

    pub fn find_include(
        &self,
        inc_type: IncludeType,
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

impl Default for CompileEnv {
    fn default() -> Self {
        Self::new(CompileSettings::default())
    }
}

fn update_cache_maps(env: &mut CompileEnv) {
    for &keyword in &Keyword::VARIANTS {
        if keyword.should_add(&env.settings) {
            let cached = env.cache.get_or_cache(keyword.text());
            env.cached_to_keywords.insert(cached.clone(), keyword);
            env.keyword_to_cached.insert(keyword, cached);
        }
    }

    for &encoding in &StringEnc::VARIANTS {
        if !encoding.should_add(&env.settings) {
            continue;
        }

        if let Some(prefix) = encoding.prefix() {
            let cached = env.cache.get_or_cache(prefix);
            env.cached_to_str_prefix.insert(cached, encoding);
        }
    }

    {
        use TokenKind::*;
        let mut map_preprocessor = |s: &str, pre: TokenKind| {
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
    }
}
