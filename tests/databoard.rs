// Copyright © 2025 Stephan Kunz
//! Integration tests for [`Databoard`].

#![allow(unused)]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use databoard::{Databoard, Remappings};

#[test]
fn standalone() {
	let databoard = Databoard::new();
	assert!(!databoard.contains("test"));
	assert!(databoard.get::<i32>("test").is_err());
	assert!(databoard.get::<String>("test").is_err());

	let old = databoard.set("test", 42).unwrap();
	assert_eq!(old, None);

	assert!(databoard.contains("test"));
	let current: i32 = databoard.get("test").unwrap();
	let seq = databoard.sequence_id("test").unwrap();
	assert_eq!(seq, 1);
	assert_eq!(current, 42);

	assert!(databoard.get::<String>("test").is_err());
	assert!(databoard.contains("test"));

	let old = databoard.set("test", 24).unwrap();
	assert_eq!(old, Some(42));
	let seq = databoard.sequence_id("test").unwrap();
	assert_eq!(seq, 2);
	assert!(databoard.get::<String>("test").is_err());

	assert!(databoard.delete::<String>("test").is_err());
	let deleted: i32 = databoard.delete("test").unwrap();
	assert_eq!(deleted, 24);
}

#[test]
#[ignore]
fn hierarchy() {
	let mut top = Databoard::new();
	assert!(!top.contains("test"));

	// auto-remapped level1
	let level1 = Databoard::with_parent(top.clone());

	// not remapped level1
	let other_level1 = Databoard::with(Some(top.clone()), None, false);

	let mut remappings = Remappings::default();
	remappings.add("remapping1", "test");
	remappings.add("remapping2", "test");
	let level2 = Databoard::with(Some(level1.clone()), Some(remappings), false);

	let old = top.set("test", 42).unwrap();
	assert_eq!(old, None);
}
