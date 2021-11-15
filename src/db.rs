use std::collections::HashMap;

use crate::{docs::Documentation, path::DocPath};

#[derive(Default)]
pub struct DocumentStore {
    finder: HashMap<DocPath, Documentation>,
}

impl DocumentStore {
    pub fn get(&self, path: &DocPath) -> Option<&Documentation> {
        self.finder.get(path)
    }

    pub fn insert(&mut self, path: DocPath, doc: Documentation) {
        self.finder.insert(path, doc);
    }
}

pub struct Session {
    pub path: DocPath,
    pub page: usize,
}

#[derive(Default)]
pub struct SessionStore {
    finder: HashMap<(i64, i64), Session>,
}

impl SessionStore {
    pub fn get(&self, chat_id: i64, message_id: i64) -> Option<&Session> {
        self.finder.get(&(chat_id, message_id))
    }

    pub fn insert(&mut self, chat_id: i64, message_id: i64, session: Session) {
        self.finder.insert((chat_id, message_id), session);
    }
}
