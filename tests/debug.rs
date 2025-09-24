// Copyright © 2025 Stephan Kunz
//! Integration tests for debugging functionalities.

#![allow(unused)]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::cognitive_complexity)]

use databoard::{Databoard, Remappings};

#[test]
fn remappings() {
	let mut remappings = Remappings::default();

	remappings.add("value1", "remapped1").unwrap();
	remappings.add("value2", "remapped2").unwrap();
	remappings.add("value3", "remapped3").unwrap();

	let remapping_string = std::format!("{:?}", &remappings);
	assert_eq!(
		remapping_string.as_str(),
		"Remappings { [(\"value1\", \"remapped1\"), (\"value2\", \"remapped2\"), (\"value3\", \"remapped3\")] }"
	);

	remappings.shrink_to_fit();

	let remapping_string = std::format!("{:?}", &remappings);
	assert_eq!(
		remapping_string.as_str(),
		"Remappings { [(\"value1\", \"remapped1\"), (\"value2\", \"remapped2\"), (\"value3\", \"remapped3\")] }"
	);
}

#[test]
fn databoard() {
	let mut databoard = Databoard::new();
	databoard.set("entry1", "value1").unwrap();
	databoard.set("entry2", "value2").unwrap();

	let databoard_string = format!("{:?}", &databoard);
	assert_eq!(
		databoard_string.as_str(),
		"Databoard { autoremap: false, Entries { [(key: entry1, sequence_id: 1, value: Any { .. }), (key: entry2, sequence_id: 1, value: Any { .. })] }, Remappings { [] }, parent: None }"
	);

	let mut remappings = Remappings::default();
	remappings.add("entry", "remapped").unwrap();
	let mut databoard = Databoard::with(None, Some(remappings), true);
	databoard.set("entry1", "value11").unwrap();

	let databoard_string = format!("{:?}", &databoard);
	assert_eq!(
		databoard_string.as_str(),
		"Databoard { autoremap: true, Entries { [(key: entry1, sequence_id: 1, value: Any { .. })] }, Remappings { [(\"entry\", \"remapped\")] }, parent: None }"
	);

	let parent = Databoard::new();
	let remappings = Remappings::default();
	let databoard = Databoard::with_parent(parent);

	let databoard_string = format!("{:?}", &databoard);
	assert_eq!(
		databoard_string.as_str(),
		"Databoard { autoremap: true, Entries { [] }, Remappings { [] }, parent: Databoard { autoremap: false, Entries { [] }, Remappings { [] }, parent: None } }"
	);

	let mut parent = Databoard::new();
	parent.set("p_entry", "p_value").unwrap();
	let mut remappings = Remappings::default();
	let mut databoard = Databoard::with_parent(parent);

	let databoard_string = format!("{:?}", &databoard);
	assert_eq!(
		databoard_string.as_str(),
		"Databoard { autoremap: true, Entries { [] }, Remappings { [] }, parent: Databoard { autoremap: false, Entries { [(key: p_entry, sequence_id: 1, value: Any { .. })] }, Remappings { [] }, parent: None } }"
	);
}
