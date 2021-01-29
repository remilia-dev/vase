// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;

use crate::{
    c::{
        CCompileEnv,
        CIncludeType,
        CLexer,
        FileId,
    },
    sync::{
        Arc,
        RwLock,
        RwLockUpgradableReadGuard,
        WorkQueue,
    },
    util::{
        CachedString,
        Utf8DecodeError,
    },
};

pub struct CMultiLexer {
    path_to_file_id: RwLock<HashMap<Arc<Path>, FileId>>,
    env: Arc<CCompileEnv>,
}
impl CMultiLexer {
    pub fn new(env: Arc<CCompileEnv>) -> CMultiLexer {
        // OPTIMIZATION: May be able to improve the hashmaps by using a different hasher.
        CMultiLexer {
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
            |inc_type, filename: &CachedString, curr_file: &Option<Arc<Path>>| -> FileId {
                return match self.find_or_add_include(inc_type, filename, curr_file.as_ref()) {
                    Ok(file_id) => file_id,
                    Err((path, file_id)) => {
                        work_queue.add_task((path, file_id));
                        file_id
                    },
                };
            };
        {
            let tl_lexer = thread_local::ThreadLocal::new();
            work_queue.work(&|tuple_args| {
                let (to_lex, slot) = tuple_args;

                let mut lexer = tl_lexer
                    .get_or(|| RefCell::new(CLexer::new(&self.env, &include_callback)))
                    .borrow_mut();
                let lexed_result = lexer.lex_file(slot, to_lex);
                self.env.file_id_to_tokens().set(slot, lexed_result);
            });
        }
    }

    pub fn find_or_add_include(
        &self,
        inc_type: CIncludeType,
        filename: &CachedString,
        curr_file: Option<&Arc<Path>>,
    ) -> Result<FileId, (Arc<Path>, FileId)> {
        return match self.env.find_include(inc_type, filename, curr_file) {
            Some(inc_file) => {
                let path_to_file_id = self.path_to_file_id.upgradable_read();
                if let Some(file_id) = path_to_file_id.get(&inc_file) {
                    return Ok(*file_id);
                }

                let new_file_id = self.env.file_id_to_tokens().reserve();
                let mut path_to_file_id = RwLockUpgradableReadGuard::upgrade(path_to_file_id);
                path_to_file_id.insert(inc_file.clone(), new_file_id);
                // Err signals that the outer function will need to have the file lex.
                return Err((inc_file, new_file_id));
            },
            None => {
                // Even though the include is missing, we don't return Err because there is no more processing needed.
                // Err is only returned when another file will need to be lexed.
                let missing_error = Err(CLexerError::MissingIncludeError(filename.clone()));
                eprintln!(
                    "'{}': Couldn't find include '{}' of type {:?}.",
                    curr_file.unwrap().display(),
                    filename.string(),
                    inc_type
                );
                Ok(self.env.file_id_to_tokens().push(missing_error))
            },
        };
    }
}

pub enum CLexerError {
    Utf8DecodeError(Utf8DecodeError),
    MissingIncludeError(CachedString),
    IOError(std::io::Error),
    EmptySlotError,
}
