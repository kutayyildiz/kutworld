use std::collections::HashSet;

use crate::Entity;

/// Stores entity membership for one world.
///
/// Entity IDs are world-local and must be used with the world that allocated
/// them. Iteration order is unspecified.
#[derive(Debug, Default)]
pub struct EntitySet {
    entities: HashSet<Entity>,
}

impl EntitySet {
    /// Creates an empty entity set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an entity, returning whether it was newly inserted.
    pub fn insert(&mut self, entity: Entity) -> bool {
        self.entities.insert(entity)
    }

    /// Removes an entity, returning whether it was present.
    pub fn remove(&mut self, entity: Entity) -> bool {
        self.entities.remove(&entity)
    }

    /// Returns whether the entity is present.
    pub fn contains(&self, entity: Entity) -> bool {
        self.entities.contains(&entity)
    }

    /// Returns the number of entities in the set.
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// Returns whether the set contains no entities.
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Iterates over entity IDs in unspecified order.
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.entities.iter().copied()
    }
}
