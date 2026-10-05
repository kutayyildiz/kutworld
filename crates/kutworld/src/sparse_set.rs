use std::collections::HashMap;

use crate::Entity;

/// Stores values associated with entity IDs for one world.
///
/// Entity IDs are world-local and must be used with the world that allocated
/// them. This storage primitive does not check entity liveness or component
/// scope. Iteration order is unspecified and may change after removal.
#[derive(Debug)]
pub struct SparseSet<T> {
    entities: Vec<Entity>,
    values: Vec<T>,
    indices: HashMap<Entity, usize>,
}

impl<T> SparseSet<T> {
    /// Creates an empty sparse set.
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
            values: Vec::new(),
            indices: HashMap::new(),
        }
    }

    /// Inserts a value, returning whether the entity was newly added.
    ///
    /// If the entity is already present, its original value is kept and the
    /// provided value is dropped.
    pub fn insert(&mut self, entity: Entity, value: T) -> bool {
        if self.indices.contains_key(&entity) {
            return false;
        }

        let index = self.entities.len();
        self.entities.push(entity);
        self.values.push(value);
        self.indices.insert(entity, index);
        true
    }

    /// Removes and returns the value for an entity, if present.
    pub fn remove(&mut self, entity: Entity) -> Option<T> {
        let index = self.indices.remove(&entity)?;
        self.entities.swap_remove(index);
        let value = self.values.swap_remove(index);

        if index < self.entities.len() {
            let moved_entity = self.entities[index];
            self.indices.insert(moved_entity, index);
        }

        Some(value)
    }

    /// Returns whether an entity has a value in the set.
    pub fn contains(&self, entity: Entity) -> bool {
        self.indices.contains_key(&entity)
    }

    /// Returns the value associated with an entity, if present.
    pub fn get(&self, entity: Entity) -> Option<&T> {
        let index = *self.indices.get(&entity)?;
        self.values.get(index)
    }

    /// Returns mutable access to the value associated with an entity, if present.
    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        let index = *self.indices.get(&entity)?;
        self.values.get_mut(index)
    }

    /// Returns the number of stored values.
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// Returns whether the set contains no values.
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Iterates over entity and value pairs in unspecified order.
    pub fn iter(&self) -> impl Iterator<Item = (Entity, &T)> + '_ {
        self.entities.iter().copied().zip(self.values.iter())
    }
}

impl<T> Default for SparseSet<T> {
    fn default() -> Self {
        Self::new()
    }
}
