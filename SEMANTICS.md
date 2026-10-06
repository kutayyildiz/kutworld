# Semantics

This document defines the behavioral model shared by TOML-authored rules and Rust-authored rules.

Implementations may optimize storage, indexing, scheduling, and generated code aggressively, but
those optimizations must preserve these semantics.

## Worlds

A program may contain multiple independent worlds.

Each world owns its own:

- entities
- component storage
- archetype indexes
- rule schedule
- execution cursor

A single rule invocation always operates on exactly one world.

Cross-world coordination is performed externally through world interfaces.

## World-Owned Declarations

Each inline module marked `#[kutworld::world]` is an isolated world. Components, archetypes, rules,
and initial entities declared inside a world belong only to that world. Ordinary Rust items outside
a world do not register declarations with it. The current Rust declaration implementation
recognizes component structs and type aliases only.

An ordinary Rust type can provide data for local components in more than one world through aliases:

```rust
pub mod components {
    pub struct HealthData(i32);
}

#[kutworld::world]
mod game {
    #[component]
    type Health = crate::components::HealthData;
}

#[kutworld::world]
mod combat {
    #[component]
    type Health = crate::components::HealthData;
}
```

These declarations use the same Rust data type, while each world owns independent `Health` storage.
Component type aliases should refer to nominal Rust types. KutWorld does not resolve the final alias
target. Rust checks type validity and visibility.

## Components

Components are ordinary Rust types or TOML primitive declarations.

```rust
#[kutworld::world]
mod game {
    #[component]
    struct Health(i32);

    #[component]
    struct Position(Vec2);

    #[component]
    struct Dead;
}
```

Components may contain a value or act as tags.

A component represents one logical value from the world's perspective.

Each world stores component data independently.

An entity has a component when that entity is present in the component's storage.

## Entities

Entities are opaque runtime identities.

They do not own their components directly.

Conceptually:

```text
Entity x

Health[x]   = ...
Position[x] = ...
```

Component presence determines the current shape of an entity.

### Entity IDs

Entity IDs are monotonically unique within each world.

Each world owns an independent allocator; separate worlds may allocate equal IDs. An entity ID
must remain associated with its owning world and is only meaningful when used with that world.

Destroyed IDs are never reused within their world, and allocation never wraps or reuses IDs after
exhaustion.

An entity ID is not a pointer or dereferenceable handle.

It may be:

- compared with another ID
- retained externally
- used later as a query restriction

For example:

```text
query where entity == cached_id
```

If the entity no longer exists, the query matches nothing.

Because entity IDs are never reused, generation counters are unnecessary.

## Zero-Component Entities

An entity may contain no components.

For example:

```text
Entity x = {}
```

Such entities are valid and may remain in the world indefinitely.

If no rule can query them, they may effectively become unreachable world state.

Preventing such dangling entities is the user's responsibility.

## Archetypes

An archetype is a named, maintained entity-membership index.

It does not own or duplicate component values.

```rust
#[kutworld::world]
mod game {
    #[archetype(Position, Velocity)]
    struct Moving;

    #[archetype(Position, Velocity, Renderable)]
    struct RenderableMoving;

    #[archetype(Position, Velocity, Physics)]
    struct PhysicalMoving;
}
```

These mean:

```text
Moving
    requires Position + Velocity

RenderableMoving
    requires Position + Velocity + Renderable

PhysicalMoving
    requires Position + Velocity + Physics
```

### Positive Only

Archetypes contain positive component requirements only.

Negative archetype requirements are not supported.

### At-Least Semantics

Archetypes describe minimum component presence, not exact entity shape.

```text
Moving = { Position, Velocity }
```

matches:

```text
{ Position, Velocity }
```

and also:

```text
{ Position, Velocity, Physics }
```

Therefore one entity may belong to multiple archetypes simultaneously.

### Flat Definitions

Archetypes are defined directly from components.

Valid:

```rust
#[archetype(Position, Velocity, Physics)]
struct PhysicalMoving;
```

Not supported:

```rust
#[archetype(Moving, Physics)]
struct PhysicalMoving;
```

Archetypes cannot contain other archetypes.

### Duplicate Archetypes

Two archetypes with identical component requirements are invalid.

For example:

```text
A = { Position, Velocity }
B = { Position, Velocity }
```

is a compile-time error.

### Unused Archetypes

An archetype that is never referenced by a query may produce a compiler warning.

Maintaining such an archetype introduces structural-mutation cost without accelerating any query.

## Initial Entities

Initial world population is declared in configuration:

```toml
[entities.player]
player = true
health = 20
```

World construction creates each declared entity empty, then adds its components one at a time
through the same generated component-add paths used during rule execution. There is no general
public `world.spawn()` or `world.add()` bootstrap API.

## Structural Mutation

Component presence changes one component at a time.

The structural operations are:

```text
add component
remove component
spawn entity
despawn entity
```

Changing a component's value is not a structural mutation.

### Add

Adding a component that is already present is a no-op.

Changing the value of an existing component uses `set` or mutable component access instead.

### Remove

Removing a component that is not present is a no-op.

### Immediate Mutation

Structural changes are applied immediately.

There is no semantic deferred-command buffer.

Archetype membership is updated immediately after every component addition or removal.

## Spawn

Spawn creates an empty entity only.

```text
spawn
-> {}
```

Components must then be added individually through the normal component-add path.

For example:

```text
spawn x
add Position to x
add Velocity to x
add Physics to x
```

Each addition immediately updates relevant archetype indexes.

There is no separate batch-initialization semantic.

A rule may initialize an entity spawned by that same rule invocation.

## Despawn

A rule may destroy its current queried entity or an entity spawned by that same rule invocation.

Destruction removes that entity from:

- component storages
- archetype indexes
- world entity state

A rule cannot mutate or destroy any other pre-existing entity.

## Indexed Mutation

Suppose:

```text
A = { Position, Velocity }
B = { Position, Velocity, Renderable }
C = { Position, Velocity, Physics }
```

Adding an indexed component may use a generated decision tree:

```text
add <component>

check Position && Velocity
├─ false -> stop
└─ true
   ├─ add A
   ├─ check Renderable -> add B
   └─ check Physics    -> add C
```

Example:

```text
create entity x

add Position
-> no archetype

add Velocity
-> add x to A

add Physics
-> add x to C
```

The exact decision tree depends on which component was added.

The compiler considers only archetypes affected by that component and may factor shared checks.

### Indexed Removal

Removal uses direct invalidation.

For the same archetypes:

```text
remove Position or Velocity
-> remove A
-> remove B
-> remove C

remove Renderable
-> remove B

remove Physics
-> remove C
```

Positive-only archetypes make removal simple because a missing required component immediately
invalidates membership.

## Rules

TOML and Rust rules share one semantic model.

A rule conceptually contains:

```text
query
view
guard
dependencies
structural capabilities
execution constraints
implementation
invocation mode
```

TOML and Rust differ only in authoring style.

Both lower into the same internal representation.

## Queries

A query decides which entities a rule operates on.

```rust
#[kutworld::world]
mod game {
    #[rule]
    #[query(has(Player, Health), not(Dead))]
    fn regenerate(health: &mut Health) {
        health.0 += 1;
    }
}
```

The query means:

```text
has Player
has Health
not Dead
```

The argument means:

```text
Health is writable
```

These concepts are independent:

```text
query     -> entity selection
arguments -> data access
```

Function arguments do not implicitly modify the query.

## Empty Queries

Rules must have a meaningful query.

A rule with no query, or an empty query matching every entity implicitly, is not allowed.

World-wide behavior should be expressed explicitly through world state and queryable
components/archetypes.

Every query must have a positive driving selector: a positive component requirement, a positive
archetype requirement, or an explicit entity restriction. Negative-only queries such as `not Dead`
are invalid. An entity-restricted query such as `entity == cached_id, not Dead` is driven by direct
identity lookup; it does not require a scan of all live entities.

## Rule Views

Rust function arguments define access to components.

```rust
fn move_entity(
    position: &mut Position,
    velocity: &Velocity,
) {
    ...
}
```

means:

```text
Position -> write
Velocity -> read
```

Every component argument must be guaranteed present by the rule's query.

That guarantee may be explicit:

```text
has Position
has Velocity
```

or implied through an archetype:

```text
has Moving
```

where:

```text
Moving = { Position, Velocity }
```

Optional component arguments are not supported.

A rule requiring different entity shapes should use separate query/rule structure.

## Negative Queries

Queries may require the absence of components.

```text
has Player
not Health
```

selects players without `Health`.

A negatively queried component cannot be used as a rule view argument.

For example:

```rust
#[query(has(Player), not(Health))]
fn invalid(health: &Health) {
    ...
}
```

is invalid.

The rule may instead declare that it adds `Health`.

## Archetypes in Queries

Archetypes may be used as query predicates.

```text
Moving = { Position, Velocity }
```

allows:

```rust
#[query(has(Moving), not(Dead))]
```

The maintained archetype index may then serve as the driving entity set.

Archetypes are selectors only.

They cannot appear as rule data arguments.

## Query Implication

Archetypes imply their required components.

```text
has Moving
```

implies:

```text
has Position
has Velocity
```

The compiler uses this implication for:

- validating rule arguments
- detecting redundancy
- detecting contradictions
- choosing query execution strategies

## Query Validity

Redundant query terms are compile-time errors.

For example:

```text
has Moving
has Position
```

is invalid because `Moving` already guarantees `Position`.

Contradictory queries are also compile-time errors.

For example:

```text
has Moving
not Position
```

is impossible.

Likewise:

```text
has Health
not Health
```

is invalid.

The compiler normalizes implied requirements before validating a query.

## Query Snapshot

Queries are evaluated once for each rule invocation.

The resulting entity set is fixed for that invocation.

For example:

```text
query -> [e1, e4, e9]

process e1
process e4
process e9
```

If processing `e1` changes its component structure, the current snapshot does not change.

Newly matching entities are not inserted into the already-running query.

A snapshot freezes entity selection only. It does not end or protect component borrows. Generated
Rust APIs must ensure that any live component `&` or `&mut` borrow that an operation could
invalidate has ended before `add`, `remove`, or `despawn` occurs.

## Rule Mutation Boundary

A rule may directly mutate only:

- its current queried entity
- any entity spawned by that same rule invocation

It may not directly mutate or destroy any other pre-existing entity.

This applies to:

- component writes
- component addition/removal
- despawn

Cross-entity behavior must be represented through world state and later rules.

This restriction is fundamental to the execution model.

## Structural Capabilities

Rust-authored rules explicitly declare structural behavior.

```rust
#[rule]
#[adds(Health)]
#[removes(Poison)]
#[spawns]
#[despawns]
fn resolve(...) {
    ...
}
```

TOML rules derive equivalent capabilities from their declared operations.

These declarations allow the compiler to reason about structural conflicts without analyzing
arbitrary Rust bodies.

## Structural Access

Structural mutation is stronger than ordinary component-value mutation.

For component `C`, structural access includes:

```text
add C
remove C
```

Structural access conflicts with concurrent rules based on all generated storage effects, including
component storages and shared generated state such as archetype indexes and entity allocation,
unless that state is explicitly synchronized. For example, if `Moving = { Position, Velocity }`,
adding either `Position` or `Velocity` may update the `Moving` index, so those additions conflict.

For component `C`, structural access includes conflicts with concurrent rules that:

```text
read C
write C
add C
remove C
query has(C)
query not(C)
use an archetype containing C
```

Adding `C` may:

```text
enter queries requiring C
leave queries excluding C
```

Removing `C` may:

```text
leave queries requiring C
enter queries excluding C
```

The compiler uses structural capabilities during scheduling.

## Rule Dependencies

Rules may define explicit execution order.

```rust
#[rule]
fn physics(...) {
    ...
}

#[rule]
#[depends(physics)]
fn collisions(...) {
    ...
}
```

This means:

```text
physics must complete before collisions starts
```

Dependencies express ordering only.

During `tick()`, external rules are skipped and do not block internal rules that depend on them.
This exception applies only to external rules skipped by `tick()`. External interface calls follow
the ordinary dependency and scheduling rules; they do not use this `tick()` exception.

They do not imply:

- data ownership
- success
- output flow
- conditional execution

Multiple dependencies are allowed.

Dependency cycles are compile-time errors.

If a dependency's guard prevents it from running, dependent rules may still run.

## Conflicting Rules

The compiler does not invent an execution order for conflicting rules.

For example:

```text
A: write Health
B: write Health
```

without a dependency is invalid.

The user must define the intended relationship:

```rust
#[depends(A)]
fn B(...) {
    ...
}
```

The same applies to structural conflicts.

This makes observable ordering explicit.

## Parallel Execution

Parallelization is implicit.

Rules may execute in parallel when they:

- have no required ordering relationship
- have no access conflict
- are not marked serial

There is no `#[parallelize]` directive.

Parallel execution is derived automatically from compile-time rule metadata.

## Serial Rules

A rule may opt out of parallel execution:

```rust
#[rule]
#[serial]
fn save_state(...) {
    ...
}
```

A serial rule does not overlap with any other rule in its world.

It occupies its own execution stage.

## Structural Mutation and Parallelism

Structural mutation does not automatically require `#[serial]`.

However, a structurally mutating rule may not execute in parallel with any rule whose query or
access can be invalidated by that mutation.

If such rules have no dependency defining their order, compilation fails.

Conflicting rules require an explicit dependency with `#[depends(...)]` whenever their effects
have observable ordering. `#[serial]` only gives a rule an exclusive execution stage; it does not
resolve conflicts or establish semantic order. The compiler must not invent ordering.

## Stages

The compiler derives execution stages at compile time.

A parallel stage executes conceptually as:

```text
1. evaluate guards
2. discard blocked rules
3. snapshot queries for remaining rules
4. execute remaining rules in parallel
```

Because all snapshots are created before execution starts, mutation performed by one rule cannot
alter another rule's current-stage iteration set.

## Guards

Guards decide whether rules execute.

The guard operations are:

```text
any(...)
none(...)
```

For example:

```rust
#[guard(none(Paused))]
```

means the rule executes only if no matching `Paused` state exists.

```rust
#[guard(any(Running))]
```

means the rule executes only if matching `Running` state exists.

Guards are evaluated immediately before their stage is snapshotted.

Changes made during a stage do not retroactively change another rule's guard result within that
stage.

## Rule State

Rules themselves are stateless.

Persistent state belongs in world data.

Rust-authored rules should not rely on hidden mutable state retained across invocations.

## World Cycles

Each world executes a cyclic schedule of rules.

The world keeps a cursor representing its current position within that cycle.

Execution is advanced externally.

Internal rules execute automatically when advancement reaches them.

External interface rules are caller-controlled synchronization points within the same schedule.
Each external interface is a hard schedule boundary. A parallel stage cannot span an interface,
and each interface executes alone in its own stage, without parallel execution alongside internal
rules or another interface. Internal rules are never targetable execution positions; execution may
stop only at the requested external interface or at the cycle boundary.

## External Interface Rules

External interface rules are semantically identical to internal rules.

They use the same:

```text
query
view
guard
dependencies
structural capabilities
mutation restrictions
scheduling
```

The only difference is how external rules enter normal advancement.

Internal rules execute automatically when reached.

External rules are processed according to the schedule and dependency graph during normal advancement;
they are never implicitly skipped. An external rule required by a dependency executes normally.
`tick()` is the only operation that skips external rules.

## External Advancement

Suppose the schedule is:

```text
A
B
InterfaceX
C
D
InterfaceY
E
```

Calling `InterfaceX` from the beginning advances through:

```text
A
B
InterfaceX
```

and stops after `InterfaceX`.

Calling `InterfaceY` next advances through:

```text
C
D
InterfaceY
```

and stops after `InterfaceY`.

External interfaces are hard boundaries between generated stages and each executes alone. During
normal advancement, external interfaces are not implicitly skipped; advancement follows the schedule
and dependency graph, including any external rules required by dependencies. Execution stops after
the requested interface or at the cycle boundary. Only `tick()` skips external interfaces and uses
the dependency exception described above.

Explicit manual skipping would require a separate API, such as `skip(...)`; no such API is currently
defined.

If the requested interface lies before the current cursor:

```text
1. finish the current cycle following the schedule and dependency graph
2. begin the next cycle
3. advance to the requested interface
4. execute it
```

## `tick()`

`tick()` advances the world to the end of its current cycle.

During `tick()`:

- internal rules execute normally
- all external interface rules are skipped

Skipping an external rule does not block an internal rule that depends on it.

If called from the beginning, `tick()` executes one complete internal cycle.

If called partway through a cycle, it executes only the remaining internal rules.

At completion, the cycle cursor returns to the beginning of the next cycle.

## Query Execution Strategy

Query semantics do not depend on implementation strategy.

A non-indexed query may execute by:

```text
iterate one component storage
check remaining component presence
```

An archetype-backed query may execute by:

```text
iterate maintained archetype membership
```

The rule implementation does not know which strategy was selected.

## TOML Rule Language

TOML rules use a deliberately small declarative language.

They may perform operations such as:

```text
set component value
add component
remove component
spawn entity
despawn current queried entity or an entity spawned by this rule invocation
```

Operations may use simple conditions:

```toml
[[rules.do]]
when = "$poison > 0"
set = "health"
value = "$health - $poison"
```

The expression language may contain:

```text
arithmetic:
    + - * / %

comparison:
    == != < <= > >=

boolean:
    && || !

grouping:
    (...)
```

TOML rule bodies are flat sequences of guarded operations.

They do not contain:

- nested control flow
- loops
- arbitrary function calls
- reflection
- general-purpose scripting

TOML is intended for simple local dataflow and conditions.

## Rust Rules

Rust rules provide the general-purpose implementation path.

```rust
#[rule]
#[query(has(Player, Health), not(Dead))]
#[adds(Regenerating)]
#[depends(input)]
fn regenerate(health: &mut Health) {
    // arbitrary Rust logic
}
```

Rust code may implement complex algorithms, but it remains constrained by the rule's declared
semantic contract.

The body does not bypass:

- query restrictions
- view restrictions
- structural declarations
- dependency scheduling
- current-entity restrictions

## Panic Behavior

Rules follow normal Rust panic behavior.

The runtime defines no separate rule-failure recovery mechanism.

## Unified Rule Representation

TOML and Rust rules lower into a shared representation containing at least:

```text
query
view
guard
dependencies
serial
adds
removes
spawns
despawns
invocation mode
implementation
```

Code generation operates on this representation.

## Core Model

```text
world
    isolated world state and cyclic execution

entity
    opaque unique identity

component
    one kind of entity data

archetype
    positive maintained entity index

query
    fixed entity selection for one invocation

view
    component access for current entity

structural capability
    declared component-presence mutation

rule
    query + view + capabilities + behavior

guard
    whether rule executes

dependency
    semantic execution ordering

interface
    caller-controlled rule synchronization point

tick
    advance current cycle while skipping interfaces
```

The central restriction is:

> A rule may directly mutate only its current queried entity or an entity spawned by that same rule
> invocation.
