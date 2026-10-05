//! Runtime support for generated Kutworld applications.
//!
//! [`Entity`] is an opaque identity value. It does not provide access to an
//! entity's data; worlds will allocate identities in a later step.

mod entity;

pub use entity::Entity;
