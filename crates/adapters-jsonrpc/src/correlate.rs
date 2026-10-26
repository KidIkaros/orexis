//! Request/response correlation: outstanding ids mapped to caller state.

use std::collections::HashMap;

use super::message::RequestId;

#[derive(Debug)]
pub struct Correlations<V> {
    pending: HashMap<RequestId, V>,
}

impl<V> Default for Correlations<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V> Correlations<V> {
    pub fn new() -> Self {
        Self {
            pending: HashMap::new(),
        }
    }

    pub fn insert(&mut self, id: RequestId, value: V) -> bool {
        if self.pending.contains_key(&id) {
            return false;
        }
        self.pending.insert(id, value);
        true
    }

    pub fn take(&mut self, id: &RequestId) -> Option<V> {
        self.pending.remove(id)
    }

    pub fn contains(&self, id: &RequestId) -> bool {
        self.pending.contains_key(id)
    }

    pub fn len(&self) -> usize {
        self.pending.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}

#[cfg(test)]
mod tests;
