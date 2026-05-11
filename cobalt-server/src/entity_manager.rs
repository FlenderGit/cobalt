use std::sync::atomic::{AtomicI32, Ordering};

use parking_lot::Mutex;

#[derive(Debug)]
pub struct EntityManager {
    next_id: AtomicI32,
    recycled: Mutex<Vec<i32>>,
}

impl Default for EntityManager {
    fn default() -> Self {
        Self {
            next_id: AtomicI32::new(1),
            recycled: Mutex::new(Vec::with_capacity(512)),
        }
    }
}

impl EntityManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn next_id(&self) -> i32 {
        if let Some(id) = self.recycled.lock().pop() {
            return id;
        }

        loop {
            let current = self.next_id.load(Ordering::Relaxed);
            if current >= i32::MAX {
                panic!("EntityManager overflow: no recycled IDs available");
            }
            if self
                .next_id
                .compare_exchange(current, current + 1, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                return current;
            }
        }
    }

    pub fn current_id(&self) -> i32 {
        self.next_id.load(Ordering::Relaxed)
    }

    pub fn release_id(&self, id: i32) {
        debug_assert!(id >= 1, "Tentative de libération d'un ID invalide: {id}");
        self.recycled.lock().push(id);
    }
}
