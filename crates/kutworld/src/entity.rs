/// An opaque identity for an entity.
///
/// An `Entity` identifies an entity but does not provide access to its data.
/// Equal values may occur in separate worlds, so keep an entity ID associated
/// with the world that allocated it and use it only with that world.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Entity(u64);

/// Allocates monotonically increasing entity identities for one world.
///
/// Each world keeps one allocator for its lifetime. Recreating an allocator
/// does not reset a world's valid identity sequence. Entity values must be used
/// with the world that allocated them; separate worlds may allocate equal
/// identities.
#[derive(Debug)]
pub struct EntityAllocator {
    next: Option<u64>,
}

impl EntityAllocator {
    /// Creates an allocator starting at entity identity zero.
    pub const fn new() -> Self {
        Self { next: Some(0) }
    }

    /// Allocates the next identity, or returns `None` after exhausting `u64`.
    pub fn allocate(&mut self) -> Option<Entity> {
        let id = self.next?;
        self.next = id.checked_add(1);
        Some(Entity(id))
    }
}

impl Default for EntityAllocator {
    fn default() -> Self {
        Self::new()
    }
}
