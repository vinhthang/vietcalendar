//! Wait-free `ArcSwap`-backed typed service registry.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use arc_swap::ArcSwap;

/// Single registered service instance owned by an active fiber.
#[derive(Clone)]
pub(crate) struct ServiceEntry {
    pub value: Arc<dyn Any + Send + Sync>,
    pub owner_fiber: String,
    pub type_name: &'static str,
}

/// Typed `TypeId -> Arc<T>` service container supporting wait-free concurrent reads.
///
/// Supports both sized structs and unsized trait objects (`dyn ServiceTrait`).
pub struct ServiceRegistry {
    snapshot: ArcSwap<HashMap<TypeId, ServiceEntry>>,
    write_lock: Mutex<()>,
}

impl ServiceRegistry {
    /// Creates an empty service registry.
    pub fn new() -> Self {
        Self {
            snapshot: ArcSwap::from_pointee(HashMap::new()),
            write_lock: Mutex::new(()),
        }
    }

    /// Reads a registered service snapshot wait-free without acquiring any lock.
    pub fn get<T>(&self) -> Option<Arc<T>>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let map = self.snapshot.load();
        let entry = map.get(&TypeId::of::<T>())?;
        entry.value.downcast_ref::<Arc<T>>().cloned()
    }

    /// Checks whether a service with the given `TypeId` is currently registered.
    pub fn contains(&self, type_id: TypeId) -> bool {
        self.snapshot.load().contains_key(&type_id)
    }

    /// Returns the owning fiber ID for a registered service `TypeId`, if any.
    pub fn owner_of(&self, type_id: TypeId) -> Option<String> {
        self.snapshot
            .load()
            .get(&type_id)
            .map(|e| e.owner_fiber.clone())
    }

    /// Inserts or replaces a typed service (`Arc<T>`) owned by `owner_fiber`.
    pub(crate) fn insert<T>(&self, owner_fiber: String, value: Arc<T>) -> (TypeId, &'static str)
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();
        let boxed: Arc<dyn Any + Send + Sync> = Arc::new(value);
        let _guard = self.write_lock.lock().unwrap_or_else(|e| e.into_inner());
        let mut next = (**self.snapshot.load()).clone();
        next.insert(
            type_id,
            ServiceEntry {
                value: boxed,
                owner_fiber,
                type_name,
            },
        );
        self.snapshot.store(Arc::new(next));
        (type_id, type_name)
    }

    /// Removes a typed service only if owned by `owner_fiber`.
    pub(crate) fn remove_owned(&self, type_id: TypeId, owner_fiber: &str) -> Option<&'static str> {
        let _guard = self.write_lock.lock().unwrap_or_else(|e| e.into_inner());
        let current = self.snapshot.load();
        let existing = current.get(&type_id)?;
        if existing.owner_fiber != owner_fiber {
            return None;
        }
        let type_name = existing.type_name;
        let mut next = (**current).clone();
        next.remove(&type_id);
        self.snapshot.store(Arc::new(next));
        Some(type_name)
    }
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}
