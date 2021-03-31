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
    cached_to_keywords: HashMap<CachedString, Keyword>,
    cached_to_preprocessor: HashMap<CachedString, TokenKind>,
    cached_to_str_prefix: HashMap<CachedString, StringEnc>,
    file_id_to_tokens: OnceArray<FileTokens>,
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
    pub fn cached_to_keywords(&self) -> &HashMap<CachedString, Keyword> {
        &self.cached_to_keywords
    }
    pub fn cached_to_preprocessor(&self) -> &HashMap<CachedString, TokenKind> {
        &self.cached_to_preprocessor
    }
    pub fn cached_to_str_prefix(&self) -> &HashMap<CachedString, StringEnc> {
        &self.cached_to_str_prefix
    }
    pub fn file_id_to_tokens(&self) -> &OnceArray<FileTokens> {
        &self.file_id_to_tokens
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

fn update_cache_maps(env: &mut CompileEnv) {
    for &keyword in &Keyword::VARIANTS {
        if keyword.should_add(&env.settings) {
            let cached = env.cache.get_or_cache(keyword.text());
            env.cached_to_keywords.insert(cached, keyword);
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
