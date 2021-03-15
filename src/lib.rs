use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone)]
pub struct KeyValueStore<K, V> {
    inner: Arc<RwLock<HashMap<K, (V, Option<SystemTime>)>>>,
}

impl<K, V> KeyValueStore<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    pub fn new() -> KeyValueStore<K, V> {
        KeyValueStore {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn insert(
        &mut self,
        key: K,
        value: V,
        expiration: Option<Duration>,
    ) -> Option<(V, Option<SystemTime>)> {
        let expiration = expiration.map(|duration| SystemTime::now() + duration);
        match (*self.inner).write() {
            Ok(mut map) => map.insert(key, (value, expiration)),
            Err(e) => panic!("{:?}", e),
        }
    }

    pub fn get(&mut self, key: &K) -> Option<(V, Option<SystemTime>)> {
        let now = SystemTime::now();
        let result = match (*self.inner).read() {
            Ok(map) => map.get(key).cloned(),
            Err(e) => panic!("{:?}", e),
        };
        match result {
            Some((value, Some(expiration))) => {
                if expiration < now {
                    self.remove(&key).and(None)
                } else {
                    Some((value, Some(expiration)))
                }
            }
            other => other,
        }
    }

    pub fn remove(&mut self, key: &K) -> Option<(V, Option<SystemTime>)> {
        match (*self.inner).write() {
            Ok(mut kvs) => kvs.remove(key),
            Err(e) => panic!("{:?}", e),
        }
    }
}

#[cfg(test)]
mod tests {}
