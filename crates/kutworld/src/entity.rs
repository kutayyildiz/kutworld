/// An opaque identity for an entity.
///
/// An `Entity` identifies an entity but does not provide access to its data.
/// Worlds will allocate entity identities in a later step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Entity(u64);
