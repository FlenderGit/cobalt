#[cfg(debug_assertions)]
use std::sync::atomic::AtomicU32;
use std::{
    hash::{BuildHasher, Hash, RandomState},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use dashmap::DashMap;
use tracing::info;

#[derive(Debug)]
pub struct CachedEntry<T> {
    value: Arc<T>,
    dirty: AtomicBool,
    last_access: Instant,
}

impl<T> CachedEntry<T> {
    pub fn new(entry: T) -> Self {
        CachedEntry {
            value: Arc::new(entry),
            dirty: AtomicBool::new(false),
            last_access: Instant::now(),
        }
    }

    pub fn set_dirty(&self) {
        self.dirty.store(true, Ordering::Relaxed);
    }

    pub fn val(&self) -> Arc<T> {
        Arc::clone(&self.value)
    }

    pub fn refresh_access(&mut self) {
        self.last_access = Instant::now();
    }
}

#[derive(Debug)]
pub struct Cache<K, T, S = RandomState>
where
    K: Eq + Hash + Clone,
    S: BuildHasher + Default + Clone,
{
    map: DashMap<K, CachedEntry<T>, S>,
    ttl: Option<Duration>,
    // #[cfg(debug_assertions)]
    // cache_hit: AtomicU32,
}

impl<K, T> Cache<K, T>
where
    T: Clone,
    K: Eq + Hash + Clone,
{
    pub fn new() -> Self {
        Self::with_hasher(RandomState::default())
    }
}

impl<K, T, S> Cache<K, T, S>
where
    K: Eq + Hash + Clone,
    T: Clone,
    S: BuildHasher + Default + Clone,
{
    pub fn with_hasher(hasher: S) -> Self {
        Self {
            map: DashMap::with_hasher(hasher),
            ttl: None,
        }
    }

    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = Some(ttl);
        self
    }

    pub fn get(&self, key: &K) -> Option<Arc<T>> {
        self.map.get(key).map(|e| Arc::clone(&e.value))
    }

    pub fn modify<F>(&self, key: &K, f: F) -> bool
    where
        F: FnOnce(&mut T),
    {
        if let Some(mut guard) = self.map.get_mut(key) {
            let cached = guard.value_mut();
            let value_mut = Arc::make_mut(&mut cached.value);
            f(value_mut);
            cached.dirty.store(true, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    pub fn insert(&self, key: K, value: T) {
        self.map.insert(key, CachedEntry::new(value));
    }

    pub fn insert_return(&self, key: K, value: Arc<T>) -> Arc<T> {
        self.map.insert(
            key,
            CachedEntry {
                value: Arc::clone(&value),
                dirty: AtomicBool::new(false),
                last_access: Instant::now(),
            },
        );
        info!("test3");
        value
    }

    pub fn remove(&self, key: &K) -> Option<Arc<T>> {
        self.map.remove(key).map(|(_, e)| e.value)
    }

    pub fn mark_dirty(&self, key: &K) {
        if let Some(entry) = self.map.get(key) {
            entry.set_dirty();
        }
    }

    pub fn flush_dirty<F>(&self, mut save_fn: F) -> usize
    where
        F: FnMut(&K, Arc<T>),
    {
        let mut count = 0;
        for entry in self.map.iter() {
            if entry.dirty.swap(false, Ordering::Relaxed) {
                save_fn(entry.key(), Arc::clone(&entry.value));
                count += 1;
            }
        }
        count
    }
}
