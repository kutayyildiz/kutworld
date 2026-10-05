//! Runtime support for generated Kutworld applications.
//!
//! [`Entity`] is an opaque, world-local identity value. Use it only with the
//! world that allocated it; it does not provide access to entity data.

mod entity;
mod entity_set;

pub use entity::{Entity, EntityAllocator};
pub use entity_set::EntitySet;
