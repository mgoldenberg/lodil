use err_derive::Error;

use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

pub type KeyValueStoreResult<V> = Result<Option<(V, Option<SystemTime>)>, KeyValueStoreError>;

#[derive(Debug, Error)]
pub enum KeyValueStoreError {
    #[error(display = "Acquired read lock was poisoned")]
    PoisonedReadLock,
    #[error(display = "Acquired write lock was poisoned")]
    PoisonedWriteLock,
}

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
    ) -> KeyValueStoreResult<V> {
        let expiration = expiration.map(|duration| SystemTime::now() + duration);
        let result = (*self.inner)
            .write()
            .map_err(|_| KeyValueStoreError::PoisonedWriteLock)?
            .insert(key, (value, expiration));
        Ok(result)
    }

    pub fn get(&mut self, key: &K) -> KeyValueStoreResult<V> {
        let now = SystemTime::now();
        let result = (*self.inner)
            .read()
            .map_err(|_| KeyValueStoreError::PoisonedReadLock)?
            .get(key)
            .cloned();
        if let Some((value, Some(expiration))) = result {
            if expiration < now {
                self.remove(&key).map(|_| None)
            } else {
                Ok(Some((value, Some(expiration))))
            }
        } else {
            Ok(result)
        }
    }

    pub fn remove(&mut self, key: &K) -> KeyValueStoreResult<V> {
        let result = (*self.inner)
            .write()
            .map_err(|_| KeyValueStoreError::PoisonedWriteLock)?
            .remove(key);
        Ok(result)
    }
}

#[cfg(test)]
mod tests {}
