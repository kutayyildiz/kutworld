# KutWorld

A tiny declarative entity/rule engine that compiles world definitions into specialized Rust code.

World state consists of entities with atomic components.

```toml
[components]

player = "tag"
health = "int"
poison = "int"

[entities.player]
player = true
health = 20
poison = 2
```

Rules query entities and transform their data.

```toml
[[rules]]
name = "poison"

[rules.query]
has = ["health", "poison"]

[rules.view]
components = ["health", "poison"]

[[rules.do]]
set = "health"
value = "$health - $poison"
```

TOML rules are intentionally simple: flat operations, simple conditions, and basic arithmetic.

More complex behavior can be written in Rust:

```rust
#[rule]
#[scope(Game)]
#[query(has(Player, Health), not(Dead))]
fn regenerate(health: &mut Health) {
    health.0 += 1;
}
```

TOML and Rust rules use the same underlying rule model.

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

Concepts such as health, poison, death, pause state, actions, and ownership are represented using
ordinary components and rules.

The core idea:

> Entities are identities. Components are data. Queries select. Rules transform.

For exact behavior and language rules, see [`SEMANTICS.md`](SEMANTICS.md).

For compiler, storage, indexing, and code-generation design, see
[`ARCHITECTURE.md`](ARCHITECTURE.md).
