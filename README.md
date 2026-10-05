# KutWorld

A tiny declarative entity/rule engine.

World state consists only of **entities with atomic components**.

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

Rules select entities, expose values, and optionally mutate them.

```toml
[[rules]]
name = "poison"

[rules.query]
has = ["health", "poison"]

[rules.view]
components = ["health", "poison"]

[rules.mutview]
entity = true

[[rules.do]]
set = "health"
value = "$health - $poison"
```

The model is intentionally small:

- `query` selects entities privately
- `view` exposes readable values
- `mutview` grants mutable access to the current entity
- `do` performs mutations
- `unless` disables a rule when a world query matches

Rules without `do` can act as external interfaces.

```toml
[[rules]]
name = "players"

[rules.query]
has = ["player", "health"]

[rules.view]
entity = true
components = ["health"]
```

Entity IDs may be exposed to outside callers and reused as handles.

They are **not valid component values inside the world**.

There are no:

- entity references
- object graphs
- systems
- messages or events
- getters/setters
- archetypes
- domain-specific engine operations

Pause states, questions, actions, death, poison, and similar concepts are all represented using
ordinary components and rules.

The core idea:

> Entities contain data. Queries find them. Views expose them. Rules transform them.
