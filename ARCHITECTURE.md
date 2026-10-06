# Architecture

This project generates specialized Rust code for declared worlds.

The current Rust frontend handles inline `#[kutworld::world]` modules, local component structs and
type aliases, and indexed query declarations over local components. Planned TOML declarations and rules
may extend the compiler model without changing world ownership.

The implementation favors compile-time specialization over runtime generality.

Behavioral guarantees are defined in `SEMANTICS.md`.

## Design Principle

If information is known at compile time, prefer:

```text
generated code
static dispatch
compile-time validation
specialized world layouts
specialized query paths
specialized scheduling
```

over:

```text
runtime registries
runtime component lookup
dynamic rule dispatch
runtime scheduling analysis
generic ECS machinery
```

The configuration describes the program's world structure.

The compiler turns that structure into concrete Rust.

## Planned Compilation Pipeline

Conceptually:

```text
TOML declarations ─┐
                   ├─> semantic IR
Rust declarations ─┘
                         │
                         ├─ resolve components
                         ├─ resolve indexed queries
                         ├─ normalize queries
                         ├─ validate rule views
                         ├─ derive structural access
                         ├─ build indexed query dependencies
                         ├─ generate indexed mutation paths
                         ├─ build rule dependency graph
                         ├─ detect conflicts
                         ├─ derive parallel stages
                         └─ generate worlds and APIs
```

The `#[kutworld::world]` attribute processes one inline module. It collects its local components,
then appends a specialized `World` type and implementation directly to that module. The compiler's
temporary list of component names drives generation; each generated runtime world has one concrete
sparse set field per component. Rust checks types, aliases, visibility, and names. This current path
does not use a separate declaration registry or cross-world resolution pass.

## Core Runtime

The handwritten runtime should remain small.

Initial primitives should be limited to things such as:

```text
Entity
EntityAllocator
SparseSet<T>
EntitySet
```

The core should not know about application-specific components, indexed queries, or rules.

It should also avoid:

```text
dynamic component registries
runtime component IDs
dynamic rule registries
runtime query planners
generic runtime schedulers
trait-object storage
```

unless later requirements genuinely demand them.

## Entity Storage

Each world owns an independent allocator that assigns monotonically increasing entity IDs.
Separate worlds may allocate equal IDs, so an ID must remain associated with the world that
allocated it.

Destroyed IDs are never reused within their world, and allocation never wraps or reuses IDs after
exhaustion.

The allocator therefore does not require generation counters.

Entity IDs provide identity only.

They do not provide direct component or entity access.

## Component Storage

Each component declaration belongs to one world and receives storage in that world only. A local
type alias may reuse an ordinary Rust data type in multiple worlds, while each world still has
independent component storage. Component type aliases should refer to nominal Rust types. KutWorld
does not resolve the final alias target.

The initial implementation uses sparse-set style storage:

```text
SparseSet<T>
├─ sparse: EntityId -> dense index
├─ entities[]
└─ values[]
```

This provides:

```text
lookup     -> O(1)
insert     -> approximately O(1)
remove     -> approximately O(1) through swap removal
iteration  -> dense
```

Tags may use the same representation with zero-sized values initially.

Storage strategy remains an implementation detail and may evolve.

## Generated Worlds

Every world becomes a concrete generated Rust type.

For example:

```rust
struct GameWorld {
    entities: EntityAllocator,

    position: SparseSet<Position>,
    velocity: SparseSet<Velocity>,
    physics: SparseSet<Physics>,
    health: SparseSet<Health>,

    moving: EntitySet,
    physical_moving: EntitySet,

    cursor: usize,
}
```

Another world may contain a completely different set of fields and generated functions.

There is no universal runtime `World` layout.

## World-Owned Declarations

Each inline `#[kutworld::world]` module is an isolated world. Components, indexed queries, rules, and
initial entities declared inside a world belong only to that world. Ordinary Rust items outside a
world do not register declarations with it. The current Rust declaration implementation recognizes
local component structs, type aliases, and unit-struct indexed queries over local components.

```rust
pub mod components {
    pub struct HealthData(i32);
}

#[kutworld::world]
mod game {
    #[component]
    type Health = crate::components::HealthData;

    #[indexed_query(Health)]
    struct Healthy;
}

#[kutworld::world]
mod combat {
    #[component]
    type Health = crate::components::HealthData;
}
```

Each `Health` declaration has independent storage in its world. Component aliases should refer to
nominal Rust types; KutWorld does not resolve the final alias target. Rust checks type validity and
visibility.

## Components

Local component declaration names provide compiler-level component identity. Their Rust types
provide data representation and type checking; a type alias can reuse ordinary Rust data without
sharing world storage.

Actual component values remain in world-owned storage.

The compiler knows every component available in every world.

This permits direct generated access without runtime component lookup.

The current compiler's semantic `World` contains `components` and `indexed_queries` metadata vectors.
The original `syn::ItemMod` stays in the parsing/code-generation path rather than in that semantic
model. Metadata vectors are compile-time collections: generated runtime worlds use concrete fields
for each component and indexed query. Rule metadata will be added when rules are implemented; these
vectors do not imply vector-backed runtime storage.

## Indexed Queries

Indexed queries are maintained `EntitySet`s.

They contain only entity membership.

They do not contain component values.

For:

```rust
#[indexed_query(Position, Velocity)]
struct Moving;
```

The current frontend accepts this marker only on a nongeneric unit struct. Its nonempty requirement
list contains unique, positive component names declared in the same world. Two indexed queries cannot
declare the same component set, even in a different order. The compiler validates these names
before code generation.

The generated world contains:

```rust
moving: EntitySet
```

while actual values remain in:

```rust
position: SparseSet<Position>
velocity: SparseSet<Velocity>
```

An indexed query is a named positive component query whose matching entity IDs are maintained as an
index.

## Indexed Query Dependency Map

The compiler derives which components affect which indexed queries.

Given:

```text
A = { Position, Velocity }
B = { Position, Velocity, Renderable }
C = { Position, Velocity, Physics }
```

derive:

```text
Position   -> A, B, C
Velocity   -> A, B, C
Renderable -> B
Physics    -> C
```

This map drives generated structural mutation logic.

Components absent from this map need no work to maintain indexed queries.

## Single-Component Mutation Gateway

All component-presence changes flow through generated single-component mutation paths.

Spawn does not bypass this mechanism.

Instead:

```text
spawn
-> empty entity

add Position
-> generated Position add path

add Velocity
-> generated Velocity add path

add Physics
-> generated Physics add path
```

This gives indexed query maintenance one consistent mutation gateway.

There is no separate batch-spawn initialization path.

Initial entities are declared in configuration, for example:

```toml
[entities.player]
player = true
health = 20
```

World construction creates each declared entity empty and adds its components individually through
these same generated component-add paths. There is no general public `world.spawn()` or
`world.add()` bootstrap API. Rule-driven spawning remains available through declared rule
capabilities and uses the same empty-entity and individual-add paths.

## Generated Add Paths

For every component participating in indexed queries, codegen collects the affected indexed queries and
derives the remaining membership requirements.

For:

```text
A = { Position, Velocity }
B = { Position, Velocity, Renderable }
C = { Position, Velocity, Physics }
```

adding `Position` produces logical requirements:

```text
A -> Velocity
B -> Velocity + Renderable
C -> Velocity + Physics
```

The add-update generator recursively factors the residual requirements into a decision tree:

```text
Velocity
├─ add A
├─ Renderable -> add B
└─ Physics    -> add C
```

At each node it tests the most frequent remaining component, breaking ties by component declaration
order. A presence check occurs at most once along a path, and every matching indexed query is inserted.
The just-added component is never checked again.

The generated implementation is specific to:

```text
world + component
```

rather than using one generic runtime indexed query updater.

This code-generation helper is implemented, but no generated mutation API calls it yet. Its caller
must invoke it after a successful component insertion.

## Generated Remove Paths

Removal is derived directly from the reverse indexed query dependency map.

For example:

```text
remove Position
-> remove A
-> remove B
-> remove C
```

No presence-query reevaluation is required because indexed queries contain positive requirements only.
The removal generator emits direct removals from every indexed query requiring the removed component.
Like addition, this helper is not wired into a generated mutation API yet; its caller must invoke it
after a successful component removal.

## Harmless Structural No-Ops

Generated mutation paths implement:

```text
add already-present component
-> no-op

remove absent component
-> no-op
```

Changing component values is handled separately from structural mutation.

## Indexed Query Diagnostics

The compiler should reject duplicate indexed queries.

It may warn for indexed queries that are declared but never queried.

This is useful because maintained indexed queries add structural-mutation cost even when unused.

## Query Representation

Queries are represented independently from rule views.

A query contains logical selection terms such as:

```text
has components
not components
has indexed queries
not indexed queries
entity restriction
```

Indexed query implications are expanded for validation.

Every query must have a positive driving selector: a positive component requirement, a positive
indexed query requirement, or an explicit entity restriction. Negative-only queries are invalid.
Entity-restricted queries use direct identity lookup and do not require a global live-entity scan.

For example:

```text
Moving = { Position, Velocity }
```

means:

```text
has Moving
```

implies both components are available to rule views.

## Query Validation

Before code generation, queries are normalized.

The compiler rejects:

```text
redundant requirements
contradictory requirements
missing view guarantees
empty queries
```

Examples:

```text
has Moving
has Position
```

is redundant.

```text
has Moving
not Position
```

is contradictory.

Both are compile-time errors.

## Query Planning

The compiler selects a query iteration strategy.

Without an indexed query, it may:

```text
choose a suitable component storage
iterate its dense entity set
check remaining predicates
```

With an indexed query, it may:

```text
iterate indexed query membership directly
apply remaining filters
```

When an explicit entity restriction is the positive driver, codegen checks that identity directly
against the required component storages and remaining predicates.

No generic runtime query planner is needed.

## Query Snapshots

Generated execution code snapshots rule query results before that rule or stage begins mutation.

For a parallel stage:

```text
evaluate guards
build query snapshots
execute stage
```

The runtime does not continuously maintain active iterators against mutable query membership.
A snapshot freezes entity selection only; it does not make live component borrows safe across
structural changes. Generated Rust APIs must structurally prevent `add`, `remove`, or `despawn` from
being called while the operation could invalidate a live component `&` or `&mut` borrow.

## Semantic Model

The compiler model is organized around worlds and their declarations:

```text
World
├─ components
├─ indexed_queries
└─ rules

Component
├─ local declaration name
└─ Rust data type

IndexedQuery
└─ required component names

Rule
├─ query and component access
├─ guard and dependencies
├─ structural capabilities
└─ implementation
```

Indexed query declarations and generated empty `EntitySet` fields are implemented. Rules and the code
that maintains indexed query membership are not implemented yet. Generated component and indexed query
storage is specialized inside each world.

## Rust Rule Metadata

Rust rules provide semantic metadata through attributes and function signatures.

For example:

```rust
#[rule]
#[query(has(Player, Health))]
#[adds(Regenerating)]
#[depends(input)]
fn regenerate(health: &mut Health) {
    ...
}
```

The compiler derives:

```text
query access
read/write access
structural access
rule ordering
```

without inspecting arbitrary Rust behavior.

## TOML Rule Compilation

TOML rules use a small fixed operation language.

The compiler can derive capabilities directly from the rule body.

For example:

```toml
[[rules.do]]
add = "dead"
```

implies:

```text
adds Dead
```

TOML does not require duplicate structural metadata when the compiler can derive it.

## TOML Expressions

TOML expressions are intentionally limited.

They may support:

```text
component-value references
literals
basic arithmetic
comparison
boolean expressions
parentheses
```

TOML bodies remain flat.

Complex algorithms belong in Rust rules.

## Access Analysis

The compiler derives component access classes:

```text
&Component
-> read

&mut Component
-> write

#[adds(Component)]
#[removes(Component)]
-> structural
```

Structural access also affects queries and indexed queries that depend on the component.

This information is used to construct scheduling conflicts.

## Current-Entity Restriction

Generated rule APIs do not expose arbitrary world mutation.

A rule may mutate:

```text
current queried entity
any entity spawned by that same rule invocation
```

but not any other pre-existing entity.

This greatly simplifies:

```text
alias analysis
parallel scheduling
query stability
destruction safety
structural conflict analysis
```

and is a deliberate architectural restriction.

The rule may add or remove components on those same entities and may despawn either the current
queried entity or an entity spawned by that invocation. It may not add, remove, or despawn any
other pre-existing entity.

## Spawn and Despawn Analysis

`#[spawns]` and `#[despawns]` are part of rule metadata.

Spawn creates an empty entity and then uses standard component-add code paths.

Despawn may target the current queried entity or an entity spawned by that same rule invocation;
it cannot target another pre-existing entity. Despawn can invalidate component and indexed query
membership and is handled conservatively by scheduling analysis.

## Dependency Graph

Explicit dependencies form a DAG.

```rust
#[depends(physics)]
fn collisions(...) {}
```

creates:

```text
physics -> collisions
```

Cycles are compile-time errors.

The dependency graph provides semantic ordering. During `tick()`, external rules are skipped and
do not block internal rules that depend on them. This exception applies only to external rules
skipped by `tick()`. Ordinary external-interface calls follow the dependency graph and scheduling
rules without this `tick()` exception.

## Conflict Graph

The compiler separately derives rule conflicts from:

```text
reads
writes
structural mutations
queries
indexed query dependencies
spawn/despawn capabilities
serial constraints
```

Conflicting rules with observable effects require an explicit dependency even when serially isolated.
The compiler does not invent ordering between conflicting rules.

Conflict analysis includes all generated storage writes, including indexes for indexed queries and
entity allocation, unless that state is explicitly synchronized. For example, when
`Moving = { Position, Velocity }`, adding `Position` and adding `Velocity` both may update the
`Moving` index, so those rules conflict even though they write different component storages.

## Stage Generation

The dependency and conflict graphs are lowered into execution stages.

For example:

```text
A -> C
B independent
D independent
```

may produce:

```text
stage 0:
    A
    B
    D

stage 1:
    C
```

Rules in one stage may execute in parallel.

External interfaces are exclusive schedule boundaries. Each interface executes alone in its own
stage, with no parallel execution alongside an internal rule or another interface. Stages cannot
span an interface. Execution positions are targetable only at an external interface or the cycle
boundary; internal rules always execute automatically while advancing to one of those positions.

## Automatic Parallelization

Parallel execution is generated automatically.

Users specify:

```text
dependencies
access
structural capabilities
serial constraints
```

The compiler derives parallelism.

There is no explicit positive parallelization directive.

## Serial Rules

`#[serial]` causes the rule to receive an exclusive stage. It does not establish ordering between
conflicting rules; observable conflicts still require explicit dependencies.

This is generated directly into the schedule rather than checked dynamically at runtime.

## Guard Generation

Guards such as:

```rust
#[guard(any(Running))]
#[guard(none(Paused))]
```

are generated as world-level existence checks.

For each execution stage:

```text
evaluate guards
discard blocked rules
snapshot remaining queries
execute stage
```

Guard evaluation does not require a generic runtime guard engine.

## World Cycle

The generated schedule is cyclic.

Each world maintains a cursor into that schedule.

The cursor allows execution to resume from the point where a previous interface call stopped.

The execution position may only stop at the requested external interface boundary or at the cycle
boundary.
Internal rules are never addressable targets and are always executed automatically while advancing
between external boundaries.

## External Interfaces

External interface rules use the same generated rule implementation as internal rules.

The difference is scheduling metadata:

```text
internal
-> automatically execute when reached

external
-> process during normal advancement according to the schedule and dependency graph
```

Generated external methods advance the world cursor until the requested interface is reached.

During normal advancement, external interfaces are never implicitly skipped. Advancement follows the
schedule and dependency graph, including any external rules required by dependencies. Execution stops
only after the requested interface or at the cycle boundary. Each interface remains an exclusive
stage boundary. Only `tick()` skips external interfaces and uses the dependency exception described
above.

Explicit manual skipping would require a separate API, such as `skip(...)`; no such API is currently
defined.

The execution position may only stop at the requested external interface boundary or at the cycle
boundary.
Internal rules are never addressable targets and are always executed automatically while advancing
between external boundaries.

## Interface Cursor Advancement

For:

```text
A
B
X(external)
C
D
Y(external)
E
```

calling `X` generates behavior equivalent to:

```text
run A
run B
run X
stop
```

Calling `Y` next:

```text
run C
run D
run Y
stop
```

Calling `X` again:

```text
run E
finish cycle

run A
run B
run X
stop
```

This behavior is compiled into the world's generated interface methods.

## `tick()`

Every world may expose:

```rust
tick()
```

`tick()` advances to the end of the current cycle while skipping all external interface rules.
Those skipped external rules do not block internal dependents; this exception applies only to
external rules skipped by `tick()`.

If called from the cycle start, it executes one complete internal cycle.

If called after partial advancement, it executes only the remaining internal rules.

## Generated Public API

The public world API consists primarily of:

```text
tick()
external rule interfaces
```

Initial entities are populated only from declarative configuration during construction. There is
no general public spawn or component-add bootstrap method; rule-driven spawning is exposed only
through the generated rule execution context.

Storage internals and structural mutation helpers remain private.

Rules themselves receive only the capabilities defined by their generated execution context.

## Generated Private API

The compiler may generate private helpers such as:

```text
component insert/remove paths
indexed mutation functions
indexed query membership updates
query runners
stage runners
cursor advancement
guard checks
spawn/despawn helpers
```

These functions may be aggressively specialized per world and component.

Their shape is not part of the user-facing API.

## No Runtime World Reflection

The generated world does not need runtime reflection over:

```text
component names
rule names
indexed query definitions
dependency graphs
```

These are compiler concerns.

If reflection is later introduced, it should be an explicit feature rather than a requirement of the
base runtime.

## Design Summary

The architecture can be summarized as:

```text
declarations
    ↓
semantic IR
    ↓
compile-time analysis
    ↓
world-specific generated Rust
    ↓
small static runtime
```

The generated world contains:

```text
component sparse sets
indexed query entity indexes
specialized mutation paths
specialized queries
compiled scheduling stages
cycle cursor
external interface methods
```

The handwritten runtime contains only the generic storage and identity primitives needed by that
generated code.

The central implementation principle is:

> Do as much work as possible once at compile time so runtime execution becomes direct, specialized
> data operations.
