use std::collections::BTreeSet;

use serde_json::Value;

const INVENTORY: &str = include_str!("../component-inventory.json");
const UI_LIB: &str = include_str!("../../component/src/lib.rs");
const STORIES_MOD: &str = include_str!("../../story/src/stories/mod.rs");

#[test]
fn every_public_component_and_story_is_accounted_for() {
    let inventory = Inventory::load();

    let expected = public_ui_modules(UI_LIB)
        .into_iter()
        .map(|name| ("ui".to_owned(), name.to_owned()))
        .chain(
            public_story_modules(STORIES_MOD)
                .into_iter()
                .map(|name| ("story".to_owned(), name.to_owned())),
        )
        .collect::<BTreeSet<_>>();

    assert_eq!(
        inventory.entries.len(),
        inventory.sources.len(),
        "duplicate inventory item"
    );
    assert_eq!(
        inventory.sources, expected,
        "inventory drifted from public exports"
    );
}

#[test]
fn inventory_entries_have_a_registration_or_a_reason() {
    for entry in Inventory::load().entries {
        match entry.classification.as_str() {
            "component" | "platform" => match entry.registration.as_ref() {
                Some(Registration::Registered {
                    descriptor,
                    exports,
                    related,
                    states,
                }) => {
                    assert!(
                        !descriptor.is_empty(),
                        "registered descriptor cannot be empty"
                    );
                    assert!(!exports.is_empty(), "registered exports cannot be empty");
                    for companion in related {
                        assert!(
                            !companion.descriptor.is_empty(),
                            "related descriptor cannot be empty"
                        );
                        assert!(
                            !companion.exports.is_empty(),
                            "related exports cannot be empty"
                        );
                        assert!(!companion.role.is_empty(), "related role cannot be empty");
                    }
                    for state in states {
                        assert!(!state.export.is_empty(), "state export cannot be empty");
                        assert!(!state.kind.is_empty(), "state kind cannot be empty");
                        assert!(!state.role.is_empty(), "state role cannot be empty");
                    }
                }
                None => panic!(
                    "{}:{} needs an explicit registered status",
                    entry.source, entry.name
                ),
            },
            "infrastructure" => assert!(
                entry.explanation.is_some_and(|value| !value.is_empty()),
                "{}:{} needs an infrastructure explanation",
                entry.source,
                entry.name
            ),
            other => panic!(
                "{}:{} has unknown classification {other}",
                entry.source, entry.name
            ),
        }
    }
}

#[test]
fn registered_inventory_matches_the_frozen_component_catalog() {
    let inventory = Inventory::load();
    let frozen = gpui_component_shell::components().expect("frozen component catalog");
    let actual = frozen
        .descriptors()
        .map(|descriptor| {
            (
                descriptor.name().to_owned(),
                descriptor
                    .constructors()
                    .iter()
                    .map(|constructor| constructor.export().to_owned())
                    .collect::<BTreeSet<_>>(),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let actual_states = frozen
        .states()
        .map(|state| (state.export().to_owned(), state.kind().to_owned()))
        .collect::<std::collections::BTreeMap<_, _>>();

    let mut inventoried_descriptors = BTreeSet::new();
    let mut inventoried_exports = BTreeSet::new();
    let mut inventoried_states = BTreeSet::new();
    for entry in inventory.entries {
        let Some(Registration::Registered {
            descriptor,
            exports,
            related,
            states,
        }) = entry.registration
        else {
            continue;
        };
        let actual_exports = actual.get(&descriptor).unwrap_or_else(|| {
            panic!(
                "{}:{} claims missing descriptor `{descriptor}`",
                entry.source, entry.name
            )
        });
        let claimed_exports = exports.into_iter().collect::<BTreeSet<_>>();
        assert_eq!(
            &claimed_exports, actual_exports,
            "{}:{} has stale exports for `{descriptor}`",
            entry.source, entry.name
        );
        inventoried_descriptors.insert(descriptor);
        inventoried_exports.extend(claimed_exports);
        for companion in related {
            let actual_exports = actual.get(&companion.descriptor).unwrap_or_else(|| {
                panic!(
                    "{}:{} claims missing related descriptor `{}`",
                    entry.source, entry.name, companion.descriptor
                )
            });
            let claimed_exports = companion.exports.into_iter().collect::<BTreeSet<_>>();
            assert_eq!(
                &claimed_exports, actual_exports,
                "{}:{} has stale exports for related `{}` ({})",
                entry.source, entry.name, companion.descriptor, companion.role
            );
            inventoried_descriptors.insert(companion.descriptor);
            inventoried_exports.extend(claimed_exports);
        }
        for state in states {
            let actual_kind = actual_states.get(&state.export).unwrap_or_else(|| {
                panic!(
                    "{}:{} claims missing retained state `{}`",
                    entry.source, entry.name, state.export
                )
            });
            assert_eq!(
                &state.kind, actual_kind,
                "{}:{} has stale kind for state `{}` ({})",
                entry.source, entry.name, state.export, state.role
            );
            inventoried_states.insert(state.export);
        }
    }

    let actual_descriptors = actual.keys().cloned().collect::<BTreeSet<_>>();
    let actual_exports = actual.values().flatten().cloned().collect::<BTreeSet<_>>();
    let actual_state_exports = actual_states.keys().cloned().collect::<BTreeSet<_>>();
    assert_eq!(
        inventoried_descriptors, actual_descriptors,
        "registered descriptors must all be inventoried"
    );
    assert_eq!(
        inventoried_exports, actual_exports,
        "registered constructor exports must all be inventoried"
    );
    assert_eq!(
        inventoried_states, actual_state_exports,
        "registered retained-state exports must all be inventoried"
    );
}

struct Inventory {
    entries: Vec<Entry>,
    sources: BTreeSet<(String, String)>,
}

struct Entry {
    source: String,
    name: String,
    classification: String,
    registration: Option<Registration>,
    explanation: Option<String>,
}

enum Registration {
    Registered {
        descriptor: String,
        exports: Vec<String>,
        related: Vec<RelatedRegistration>,
        states: Vec<StateRegistration>,
    },
}

struct RelatedRegistration {
    descriptor: String,
    exports: Vec<String>,
    role: String,
}

struct StateRegistration {
    export: String,
    kind: String,
    role: String,
}

impl Inventory {
    fn load() -> Self {
        let document: Value = serde_json::from_str(INVENTORY).expect("valid component inventory");
        let entries = document["items"]
            .as_array()
            .expect("inventory items array")
            .iter()
            .map(|item| Entry {
                source: item["source"]
                    .as_str()
                    .expect("inventory item source")
                    .to_owned(),
                name: item["name"]
                    .as_str()
                    .expect("inventory item name")
                    .to_owned(),
                classification: item["classification"]
                    .as_str()
                    .expect("inventory item classification")
                    .to_owned(),
                registration: item.get("registration").map(|registration| {
                    let status = registration["status"]
                        .as_str()
                        .expect("registration status");
                    match status {
                        "registered" => Registration::Registered {
                            descriptor: registration["descriptor"]
                                .as_str()
                                .expect("registered descriptor")
                                .to_owned(),
                            exports: registration["exports"]
                                .as_array()
                                .expect("registered exports")
                                .iter()
                                .map(|export| {
                                    export.as_str().expect("registered export").to_owned()
                                })
                                .collect(),
                            related: registration
                                .get("related")
                                .and_then(Value::as_array)
                                .into_iter()
                                .flatten()
                                .map(|related| RelatedRegistration {
                                    descriptor: related["descriptor"]
                                        .as_str()
                                        .expect("related descriptor")
                                        .to_owned(),
                                    exports: related["exports"]
                                        .as_array()
                                        .expect("related exports")
                                        .iter()
                                        .map(|export| {
                                            export.as_str().expect("related export").to_owned()
                                        })
                                        .collect(),
                                    role: related["role"]
                                        .as_str()
                                        .expect("related role")
                                        .to_owned(),
                                })
                                .collect(),
                            states: registration
                                .get("states")
                                .and_then(Value::as_array)
                                .into_iter()
                                .flatten()
                                .map(|state| StateRegistration {
                                    export: state["export"]
                                        .as_str()
                                        .expect("state export")
                                        .to_owned(),
                                    kind: state["kind"].as_str().expect("state kind").to_owned(),
                                    role: state["role"].as_str().expect("state role").to_owned(),
                                })
                                .collect(),
                        },
                        other => panic!("unknown registration status {other}"),
                    }
                }),
                explanation: item["explanation"].as_str().map(ToOwned::to_owned),
            })
            .collect::<Vec<_>>();
        let sources = entries
            .iter()
            .map(|entry| (entry.source.clone(), entry.name.clone()))
            .collect();

        Self { entries, sources }
    }
}

fn public_ui_modules(source: &str) -> BTreeSet<&str> {
    source
        .lines()
        .filter_map(|line| line.trim().strip_prefix("pub mod "))
        .filter_map(module_name)
        .collect()
}

fn public_story_modules(source: &str) -> BTreeSet<&str> {
    source
        .lines()
        .filter_map(|line| line.trim().strip_prefix("pub use "))
        .filter_map(|line| line.split_once("::"))
        .map(|(module, _)| module.trim_end_matches("_story"))
        .collect()
}

fn module_name(line: &str) -> Option<&str> {
    line.split([' ', '{', ';'])
        .next()
        .filter(|name| !name.is_empty())
}
