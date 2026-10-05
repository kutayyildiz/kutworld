//! Runtime support for generated Kutworld applications.
//!
//! [`Entity`] is an opaque, world-local identity value. Use it only with the
//! world that allocated it; it does not provide access to entity data.

mod entity;

pub use entity::{Entity, EntityAllocator};
