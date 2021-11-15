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
    finder: HashMap<i64, Session>,
}

impl SessionStore {
    pub fn get(&self, user_id: i64) -> Option<&Session> {
        self.finder.get(&user_id)
    }

    pub fn insert(&mut self, user_id: i64, session: Session) {
        self.finder.insert(user_id, session);
    }
}
