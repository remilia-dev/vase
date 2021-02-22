// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;

use crate::{
    c::{
        CompileEnv,
        FileId,
        IncludeType,
        Lexer,
    },
    sync::{
        Arc,
        RwLock,
        RwLockUpgradableReadGuard,
        WorkQueue,
    },
    util::CachedString,
};

pub struct MultiLexer {
    path_to_file_id: RwLock<HashMap<Arc<Path>, FileId>>,
    env: Arc<CompileEnv>,
}
impl MultiLexer {
    pub fn new(env: Arc<CompileEnv>) -> MultiLexer {
        // OPTIMIZATION: May be able to improve the hashmaps by using a different hasher.
        MultiLexer {
            path_to_file_id: RwLock::new(HashMap::new()),
            env,
        }
    }

    pub fn lex_multi_threaded(&mut self, files: &[Arc<Path>]) {
        let mut work_queue = WorkQueue::<(Arc<Path>, FileId)>::new(self.env.threads());
        work_queue.add_tasks_mut(
            files
                .iter()
                .map(|file| (file.clone(), self.env.file_id_to_tokens().reserve())),
        );

        let include_callback =
            |inc_type, filename: &CachedString, curr_file: &Option<Arc<Path>>| -> Option<FileId> {
                let (path, file_id) =
                    self.find_or_add_include(inc_type, filename, curr_file.as_ref());
                if let Some(path) = path {
                    work_queue.add_task((path, file_id.unwrap()));
                }
                file_id
            };
        {
            let tl_lexer = thread_local::ThreadLocal::new();
            work_queue.work(&|tuple_args| {
                let (to_lex, file_id) = tuple_args;

                let mut lexer = tl_lexer
                    .get_or(|| RefCell::new(Lexer::new(&self.env, &include_callback)))
                    .borrow_mut();
                let tokens = lexer.lex_file(file_id, to_lex);
                self.env.file_id_to_tokens().set(file_id, tokens.into());
            });
        }
    }

    pub fn find_or_add_include(
        &self,
        inc_type: IncludeType,
        filename: &CachedString,
        curr_file: Option<&Arc<Path>>,
    ) -> (Option<Arc<Path>>, Option<FileId>) {
        match self.env.find_include(inc_type, filename, curr_file) {
            Some(inc_file) => {
                let path_to_file_id = self.path_to_file_id.upgradable_read();
                if let Some(file_id) = path_to_file_id.get(&inc_file) {
                    return (None, Some(*file_id));
                }

                let new_file_id = self.env.file_id_to_tokens().reserve();
                let mut path_to_file_id = RwLockUpgradableReadGuard::upgrade(path_to_file_id);
                path_to_file_id.insert(inc_file.clone(), new_file_id);
                // Err signals that the outer function will need to have the file lex.
                (Some(inc_file), Some(new_file_id))
            },
            None => (None, None),
        }
    }
}
