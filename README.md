# KutWorld

A tiny declarative entity/rule engine that compiles world definitions into specialized Rust code.

Each world is an inline module marked with `#[kutworld::world]`. Its components belong only to that
world. Ordinary Rust component data can be reused by local aliases that name paths such as
`crate::components::HealthData`.

```rust
pub mod components {
    pub struct HealthData(pub i32);
}

#[kutworld::world]
pub mod game {
    #[component]
    pub type Health = crate::components::HealthData;

    #[component]
    pub struct Player;

    #[archetype(Health, Player)]
    pub struct AlivePlayer;
}

#[kutworld::world]
pub mod combat {
    #[component]
    pub type Health = crate::components::HealthData;
}

fn main() {
    let _game = game::World::new();
    let _combat = combat::World::default();
}
```

Component aliases should refer to nominal Rust types. KutWorld does not resolve the final alias
target. The current declaration support recognizes local component structs and type aliases, then
recognizes unit-struct archetypes with local component requirements. It generates component storage,
empty archetype indexes, and `World::new()` / `Default` constructors. Rules, initial entities,
archetype membership maintenance, and execution scheduling are not implemented yet.
Archetype markers currently require nongeneric unit structs with nonempty, unique, local component
requirements; identical requirement sets are rejected regardless of order.

KutWorld is intentionally data-oriented:

- entities are opaque IDs
- components hold data
- queries select entities
- rules operate on the current entity
- archetypes provide maintained query indexes
- dependencies define rule ordering
- safe parallelism is derived at compile time
- worlds are specialized through code generation

There are no domain-specific engine operations.

For exact behavior and language rules, see [`SEMANTICS.md`](SEMANTICS.md).

For compiler, storage, indexing, and code-generation design, see
[`ARCHITECTURE.md`](ARCHITECTURE.md).
