#![feature(external_doc)]
#![doc(include = "../README.md")]

use err_derive::Error;

use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

/// Convenient type definition returned by all [`KeyValueStore`] methods.
pub type KeyValueStoreResult<V> = Result<Option<V>, KeyValueStoreError>;

/// Error types that can be returned from [`KeyValueStore`] methods.
/// These have to do with the possibility of having poisoned locks
/// when a thread holding a lock panics.
#[derive(Debug, Eq, PartialEq, Error)]
pub enum KeyValueStoreError {
    /// Returned when a read lock is acquired but is poisoned.
    #[error(display = "Acquired read lock was poisoned")]
    PoisonedReadLock,
    /// Returned when a write lock is acquired but is poisoned.
    #[error(display = "Acquired write lock was poisoned")]
    PoisonedWriteLock,
}

/// Primary structure in this library. It is a general-purpose,
/// key-value store that is thread safe and allows one to set
/// expiration times on entries. It's primary purpose is to
/// wrap an `Arc<RwLock<HashMap>>` and expose a limited set of
/// functions for inserting, removing, and retrieving values
/// by key.
#[derive(Debug, Clone)]
pub struct KeyValueStore<K, V> {
    inner: Arc<RwLock<HashMap<K, (V, Option<SystemTime>)>>>,
}

impl<K, V> KeyValueStore<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    /// Create a new, empty [`KeyValueStore`].
    pub fn new() -> KeyValueStore<K, V> {
        KeyValueStore {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Insert a key with an associated value and an optional
    /// expiration. Expiration values are provided in the form
    /// of a [`std::time::Duration`], which will be used to
    /// determine an absolute time after which the key will be
    /// invalidated. Note that keys are lazily invalidated, i.e.
    /// they are removed from the underlying map when calling
    /// [`KeyValueStore::get`] after the `Duration` has elapsed.
    /// If the key already exists, the associated value and expiration
    /// time are updated with those given and the previous values
    /// are returned in the [`KeyValueStoreResult`].
    ///
    /// Calling this function will always cause it to attempt to
    /// hold a write lock on the underlying `HashMap`, which means
    /// that no other locks can be obtained.
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
        Ok(result.map(|(value, _)| value))
    }

    /// Get the value associated with the given key. Returns `Ok(None)`
    /// if the key was not found. This function will also return `Ok(None)`
    /// if the value was found, but has expired, at which point the entry
    /// will also be removed from the underlying map. For this reason
    /// the function takes `&mut self` rather than `&self`.
    /// 
    /// Calling this function will always cause it to attempt to
    /// hold a read lock on the underlying `HashMap`, which means
    /// that other read locks can be obtained, but a write lock cannot
    /// be obtained. However, if the value retrieved has expired,
    /// the read lock is released and the function will call
    /// [`KeyValueStore::remove`] which tries to obtain a write lock.
    pub fn get(&mut self, key: &K) -> KeyValueStoreResult<V> {
        let now = SystemTime::now();
        let result = (*self.inner)
            .read()
            .map_err(|_| KeyValueStoreError::PoisonedReadLock)?
            .get(key)
            .cloned();
        if let Some((value, Some(expiration))) = result {
            if expiration < now {
                // This doesn't create a dead write lock
                // because the read lock has been already
                // been released.
                self.remove(&key).map(|_| None)
            } else {
                Ok(Some(value))
            }
        } else {
            Ok(result.map(|(value, _)| value))
        }
    }

    /// Remove the entry associated with the given key. Returns
    /// the value, if the key was found, otherwise returns `Ok(None)`.
    ///
    /// Calling this function will always cause it to attempt to
    /// hold a write lock on the underlying `HashMap`, which means 
    /// no other locks can be obtained.
    pub fn remove(&mut self, key: &K) -> KeyValueStoreResult<V> {
        let result = (*self.inner)
            .write()
            .map_err(|_| KeyValueStoreError::PoisonedWriteLock)?
            .remove(key);
        Ok(result.map(|(value, _)| value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn insert_and_get() {
        let mut kvs = KeyValueStore::new();
        let (key, value) = (1, 1);
        assert_eq!(kvs.insert(key, value, None), Ok(None));
        assert_eq!(kvs.get(&key), Ok(Some(value)));
    }

    #[test]
    fn update_and_get() {
        let mut kvs = KeyValueStore::new();
        let (key, value, update) = (1, 1, 2);
        assert_eq!(kvs.insert(key, value, None), Ok(None));
        assert_eq!(kvs.insert(key, update, None), Ok(Some(value)));
        assert_eq!(kvs.get(&key), Ok(Some(update)));
    }

    #[test]
    fn empty_get() {
        let mut kvs = KeyValueStore::<_, ()>::new();
        assert_eq!(kvs.get(&1), Ok(None));
    }

    #[test]
    fn expired() {
        let mut kvs = KeyValueStore::new();
        let (key, value, expiration) = (1, 1, Duration::from_secs(1));
        assert_eq!(kvs.insert(key, value, Some(expiration)), Ok(None));
        assert_eq!(kvs.get(&key), Ok(Some(value)));
        thread::sleep(expiration);
        assert_eq!(kvs.get(&key), Ok(None));
    }

    #[test]
    fn synchronized_insert_and_get() {
        let mut kvs = KeyValueStore::new();
        let (key, value) = (1, 1);

        let cloned = (kvs.clone(), key.clone(), value.clone());
        let handle = thread::spawn(|| {
            let (mut kvs, key, value) = cloned;
            thread::sleep(Duration::from_secs(1));
            assert_eq!(kvs.insert(key, value, None), Ok(Some(1)));
        });

        assert_eq!(kvs.get(&key), Ok(None));
        loop {
            if let Ok(Some(v)) = kvs.get(&key) {
                assert_eq!(v, value);
                break;
            } else {
                thread::sleep(Duration::from_secs(1));
            }
        }
        handle.join().expect("should join without error");
    }
}
