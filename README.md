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

    #[indexed_query(Health, Player)]
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
recognizes unit-struct indexed queries with local component requirements. It generates component storage,
empty indexes for declared indexed queries, private component add/remove methods, and `World::new()` /
`Default` constructors. Rules, initial entities, public mutation APIs, and execution scheduling are
not implemented yet.
Indexed query markers currently require nongeneric unit structs with nonempty, unique, local component
requirements; identical requirement sets are rejected regardless of order.

External IRs are the sole coupling point between external code and a world; world internals remain
private. The conceptual generated API exposes only each IR's declared query and allowed operations:
prepare at its schedule boundary, iterate IR-specific entity capabilities, then sync to close the
interaction. This public integration API is not implemented yet.

## Planned Hybrid Query and Storage Model

Planned, not implemented: sparse component storage remains the default; queries may opt into
archetype-table optimization without changing semantics. This differs from supported indexed
queries.

Optimized storage has one table per exact component set. A positive query `has [A, C]` matches every
table containing both, such as `{A, C}`, `{A, B, C}`, `{A, C, D}`, and `{A, B, C, D}`. Codegen
precomputes matching tables for direct bulk traversal of component columns aligned within each
table, without per-entity sparse lookups. Tables have independent row order; each entity occupies
only its own table, so overlapping queries do not duplicate component values. Non-opted components
and queries keep the regular path.

An internal location index maps an opaque entity ID to its current storage location, such as a table
and row. `physics.filtered_iter([id1, id2, id3])` resolves each ID, checks membership in the External
IR's query, and yields the same generated item type as normal iteration. A list of N IDs takes
approximately N direct indexed lookups rather than scanning all matches; this is not
`Iterator::filter` over a bulk query. `physics.filtered_iter([id])` supports a single targeted ID.
With archetype optimization enabled, `physics.iter()` walks matching tables sequentially for
locality; otherwise it uses the regular query path. Filtering only narrows selection, adding no
access capability or generic `World::get_entity`.

One canonical internal add/remove path per component type will update ordinary storage, table
membership, entity location, and affected indexed-query entries. Internal `AddComponent<T>` and
`RemoveComponent<T>` traits are the conceptual route used by rules and External IR wrappers. A
per-entity operation such as `entity.add_health(health)` is restricted to the current entity, not a
general external mutation API. This adds neither systems nor entity-reference graphs to the flat
model.

KutWorld is intentionally data-oriented:

- entities are opaque IDs
- components hold data
- queries select entities
- rules operate on the current entity
- indexed queries provide maintained entity-ID indexes for rule queries
- dependencies define rule ordering
- safe parallelism is derived at compile time
- worlds are specialized through code generation

There are no domain-specific engine operations.

For exact behavior and language rules, see [`SEMANTICS.md`](SEMANTICS.md).

For compiler, storage, indexing, and code-generation design, see
[`ARCHITECTURE.md`](ARCHITECTURE.md).
