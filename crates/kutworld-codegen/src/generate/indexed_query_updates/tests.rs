use super::{after_add, after_remove};
use crate::model::{Component, IndexedQuery, World};
use quote::{format_ident, quote};
use std::env;
use std::process::Command;

fn component(name: &str) -> Component {
    Component {
        name: format_ident!("{name}"),
    }
}

fn indexed_query(name: &str, components: &[&str]) -> IndexedQuery {
    IndexedQuery {
        name: format_ident!("{name}"),
        components: components
            .iter()
            .map(|name| format_ident!("{name}"))
            .collect(),
    }
}

fn fixture_world() -> World {
    World {
        components: vec![
            component("C0"),
            component("C1"),
            component("C2"),
            component("C3"),
        ],
        indexed_queries: vec![
            indexed_query("Singleton", &["C0"]),
            indexed_query("C0C1", &["C0", "C1"]),
            indexed_query("C0C1C2", &["C0", "C1", "C2"]),
            indexed_query("C0C2", &["C0", "C2"]),
            indexed_query("Unrelated", &["C1"]),
            indexed_query("C0C1C3", &["C0", "C1", "C3"]),
        ],
        initial_entities: Vec::new(),
        rules: Vec::new(),
    }
}

#[test]
fn generated_updates_match_component_sets_for_all_small_masks() {
    let world = fixture_world();
    let add = after_add(&world, &world.components[0]);
    let remove = after_remove(&world, &world.components[0]);
    let no_indexed_queries = World {
        components: vec![component("C0")],
        indexed_queries: Vec::new(),
        initial_entities: Vec::new(),
        rules: Vec::new(),
    };
    assert!(after_add(&no_indexed_queries, &no_indexed_queries.components[0]).is_empty());
    assert!(after_remove(&no_indexed_queries, &no_indexed_queries.components[0]).is_empty());

    let singleton_world = World {
        components: vec![component("C0")],
        indexed_queries: vec![indexed_query("OnlyC0", &["C0"])],
        initial_entities: Vec::new(),
        rules: Vec::new(),
    };
    let singleton_add = after_add(&singleton_world, &singleton_world.components[0]);
    assert!(!singleton_add.to_string().contains("contains"));

    let tie_world = World {
        components: vec![component("C0"), component("C1"), component("C2")],
        indexed_queries: vec![
            indexed_query("First", &["C0", "C1"]),
            indexed_query("Second", &["C0", "C2"]),
        ],
        initial_entities: Vec::new(),
        rules: Vec::new(),
    };
    let tie_code = after_add(&tie_world, &tie_world.components[0]).to_string();
    assert!(tie_code.starts_with("if self . __kutworld_component_1 . contains"));
    assert_eq!(
        add.to_string(),
        after_add(&world, &world.components[0]).to_string()
    );

    let component_fields: Vec<_> = world
        .components
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let field = format_ident!("__kutworld_component_{index}");
            quote!(#field: Store)
        })
        .collect();
    let component_initializers = world.components.iter().enumerate().map(|(index, _)| {
        let field = format_ident!("__kutworld_component_{index}");
        let initial = if index == 0 {
            quote!(true)
        } else {
            let bit = index - 1;
            quote!(mask & (1 << #bit) != 0)
        };
        quote!(#field: Store::new(#initial))
    });
    let indexed_query_fields: Vec<_> = world
        .indexed_queries
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let field = format_ident!("__kutworld_indexed_query_{index}");
            quote!(#field: Index)
        })
        .collect();
    let indexed_query_initializers = world.indexed_queries.iter().enumerate().map(|(index, _)| {
        let field = format_ident!("__kutworld_indexed_query_{index}");
        let initial = index == 4;
        quote!(#field: Index::new(#initial))
    });
    let requirement_masks = world.indexed_queries.iter().map(|indexed_query| {
        let mask = indexed_query
            .components
            .iter()
            .fold(0usize, |mask, component| {
                let index = world
                    .components
                    .iter()
                    .position(|item| item.name == *component)
                    .unwrap();
                mask | (1 << index)
            });
        quote!(#mask)
    });
    let singleton_field = format_ident!("__kutworld_component_0");
    let singleton_index = format_ident!("__kutworld_indexed_query_0");
    let source = quote! {
        use std::cell::Cell;

        const ENTITY: u64 = 42;

        struct Store { present: bool, reads: Cell<usize> }
        impl Store {
            fn new(present: bool) -> Self { Self { present, reads: Cell::new(0) } }
            fn contains(&self, entity: u64) -> bool {
                assert_eq!(entity, ENTITY);
                self.reads.set(self.reads.get() + 1);
                self.present
            }
        }
        struct Index { present: bool, insertions: usize, removals: usize }
        impl Index {
            fn new(present: bool) -> Self { Self { present, insertions: 0, removals: 0 } }
            fn insert(&mut self, entity: u64) -> bool {
                assert_eq!(entity, ENTITY);
                self.insertions += 1;
                let was_present = self.present;
                self.present = true;
                !was_present
            }
            fn remove(&mut self, entity: u64) -> bool {
                assert_eq!(entity, ENTITY);
                self.removals += 1;
                let was_present = self.present;
                self.present = false;
                was_present
            }
        }
        struct Fixture { #(#component_fields,)* #(#indexed_query_fields,)* }
        impl Fixture {
            fn new(mask: usize) -> Self {
                Self { #(#component_initializers,)* #(#indexed_query_initializers,)* }
            }
            fn after_add(&mut self, entity: u64) { #add }
            fn after_remove(&mut self, entity: u64) { #remove }
        }
        struct Singleton { #singleton_field: Store, #singleton_index: Index }
        impl Singleton {
            fn after_add(&mut self, entity: u64) { #singleton_add }
        }
        const REQUIREMENTS: &[usize] = &[#(#requirement_masks),*];
        fn main() {
            for mask in 0usize..8 {
                let entity = ENTITY;
                let mut world = Fixture::new(mask);
                world.after_add(entity);
                let after = (mask << 1) | 1;
                for (index, required) in REQUIREMENTS.iter().enumerate() {
                    let expected = if required & 1 != 0 {
                        after & required == *required
                    } else {
                        index == 4
                    };
                    let actual = match index {
                        0 => world.__kutworld_indexed_query_0.present,
                        1 => world.__kutworld_indexed_query_1.present,
                        2 => world.__kutworld_indexed_query_2.present,
                        3 => world.__kutworld_indexed_query_3.present,
                        4 => world.__kutworld_indexed_query_4.present,
                        _ => world.__kutworld_indexed_query_5.present,
                    };
                    assert_eq!(actual, expected, "mask={mask} indexed query={index}");
                }
                assert_eq!(world.__kutworld_component_0.reads.get(), 0);
                assert!(world.__kutworld_component_1.reads.get() <= 1);
                assert!(world.__kutworld_component_2.reads.get() <= 1);
                assert!(world.__kutworld_component_3.reads.get() <= 1);
                if mask & 1 == 0 {
                    assert_eq!(world.__kutworld_component_3.reads.get(), 0);
                }
                assert_eq!(world.__kutworld_indexed_query_4.insertions, 0);
                assert_eq!(world.__kutworld_indexed_query_4.removals, 0);
                let reads = [
                    world.__kutworld_component_0.reads.get(),
                    world.__kutworld_component_1.reads.get(),
                    world.__kutworld_component_2.reads.get(),
                    world.__kutworld_component_3.reads.get(),
                ];
                world.after_remove(entity);
                assert_eq!(
                    reads,
                    [
                        world.__kutworld_component_0.reads.get(),
                        world.__kutworld_component_1.reads.get(),
                        world.__kutworld_component_2.reads.get(),
                        world.__kutworld_component_3.reads.get(),
                    ]
                );
                assert!(!world.__kutworld_indexed_query_0.present);
                assert!(!world.__kutworld_indexed_query_1.present);
                assert!(!world.__kutworld_indexed_query_2.present);
                assert!(!world.__kutworld_indexed_query_3.present);
                assert!(world.__kutworld_indexed_query_4.present);
                assert!(!world.__kutworld_indexed_query_5.present);
                assert_eq!(world.__kutworld_indexed_query_4.insertions, 0);
                assert_eq!(world.__kutworld_indexed_query_4.removals, 0);
            }
            let mut singleton = Singleton {
                __kutworld_component_0: Store::new(true),
                __kutworld_indexed_query_0: Index::new(false),
            };
            singleton.after_add(ENTITY);
            assert!(singleton.__kutworld_indexed_query_0.present);
            assert_eq!(singleton.__kutworld_component_0.reads.get(), 0);
        }
    };

    let source = source.to_string();
    let directory = tempfile::tempdir().expect("create generated-code test directory");
    let source_path = directory.path().join("fixture.rs");
    let executable_path = if env::consts::EXE_EXTENSION.is_empty() {
        directory.path().join("fixture")
    } else {
        directory
            .path()
            .join(format!("fixture.{}", env::consts::EXE_EXTENSION))
    };
    std::fs::write(&source_path, source).unwrap();

    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let compile = Command::new(&rustc)
        .arg("--edition=2024")
        .arg(&source_path)
        .arg("-o")
        .arg(&executable_path)
        .output()
        .unwrap_or_else(|error| {
            panic!(
                "failed to run Rust compiler `{}` for generated-code fixture: {error}",
                rustc.to_string_lossy()
            )
        });
    assert!(
        compile.status.success(),
        "generated fixture failed to compile with {}:\nstdout:\n{}\nstderr:\n{}",
        compile.status,
        String::from_utf8_lossy(&compile.stdout),
        String::from_utf8_lossy(&compile.stderr),
    );
    let execution = Command::new(&executable_path)
        .output()
        .unwrap_or_else(|error| panic!("failed to run generated fixture: {error}"));
    assert!(
        execution.status.success(),
        "generated fixture failed with {}:\nstdout:\n{}\nstderr:\n{}",
        execution.status,
        String::from_utf8_lossy(&execution.stdout),
        String::from_utf8_lossy(&execution.stderr),
    );
}
