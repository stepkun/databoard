// Copyright © 2025 Stephan Kunz
//! Integration tests for [`Databoard`].

#![allow(clippy::cognitive_complexity)]
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

	let old = databoard.set::<i32>("test", 42).unwrap();
	assert_eq!(old, None);

	assert!(databoard.contains("test"));
	assert_eq!(databoard.sequence_id("test").unwrap(), 1);
	assert_eq!(databoard.get::<i32>("test").unwrap(), 42);

	assert!(databoard.get::<String>("test").is_err());
	assert!(databoard.contains("test"));

	assert_eq!(databoard.set::<i32>("test", 24).unwrap(), Some(42));
	assert_eq!(databoard.sequence_id("test").unwrap(), 2);
	assert!(databoard.get::<String>("test").is_err());

	assert!(databoard.delete::<String>("test").is_err());
	assert_eq!(databoard.delete::<i32>("test").unwrap(), 24);
}

#[test]
fn root_access_no_remapping() {
	let root = Databoard::new();
	let level1 = Databoard::with(Some(root.clone()), None, false);
	let level2 = Databoard::with(Some(level1.clone()), None, false);

	// set 'test' from level2 in root
	assert_eq!(level2.set("@test", 42).unwrap(), None);
	assert!(root.contains("test"));
	assert!(!level1.contains("test"));
	assert!(!level2.contains("test"));
	assert_eq!(root.get::<i32>("@test").unwrap(), 42);
	assert_eq!(root.sequence_id("@test").unwrap(), 1);
	assert_eq!(level1.get::<i32>("@test").unwrap(), 42);
	assert_eq!(level1.sequence_id("@test").unwrap(), 1);
	assert_eq!(level2.get::<i32>("@test").unwrap(), 42);
	assert_eq!(level2.sequence_id("@test").unwrap(), 1);

	// set 'test' in level2
	assert_eq!(level2.set("test", 44).unwrap(), None);
	assert!(!level1.contains("test"));
	assert_eq!(level2.get::<i32>("test").unwrap(), 44);
	assert_eq!(level2.sequence_id("test").unwrap(), 1);
	assert_eq!(level2.get::<i32>("@test").unwrap(), 42);
	assert_eq!(level2.sequence_id("@test").unwrap(), 1);

	// update 'test' from level2 in root
	assert_eq!(level2.set("@test", 24).unwrap(), Some(42));
	assert!(!level1.contains("test"));
	assert_eq!(level2.get::<i32>("@test").unwrap(), 24);
	assert_eq!(level2.sequence_id("@test").unwrap(), 2);
	assert_eq!(level2.get::<i32>("test").unwrap(), 44);
	assert_eq!(level2.sequence_id("test").unwrap(), 1);

	// update 'test' in level2
	assert_eq!(level2.set("test", 22).unwrap(), Some(44));
	assert!(!level1.contains("test"));
	assert_eq!(level2.get::<i32>("@test").unwrap(), 24);
	assert_eq!(level2.sequence_id("@test").unwrap(), 2);
	assert_eq!(level2.get::<i32>("test").unwrap(), 22);
	assert_eq!(level2.sequence_id("test").unwrap(), 2);

	// delete 'test'
	assert_eq!(level2.delete::<i32>("test").unwrap(), 22);
	assert_eq!(level2.delete::<i32>("@test").unwrap(), 24);
	assert!(!root.contains("test"));
	assert!(!level1.contains("test"));
	assert!(!level2.contains("test"));
}

#[test]
fn root_access_auto_remapping() {
	let root = Databoard::new();
	let level1 = Databoard::with_parent(root.clone());
	let level2 = Databoard::with_parent(level1.clone());

	// set 'test' from level2 in root
	assert_eq!(level2.set("@test", 42).unwrap(), None);
	assert!(root.contains("test"));
	assert!(level1.contains("test"));
	assert!(level2.contains("test"));
	assert_eq!(root.get::<i32>("@test").unwrap(), 42);
	assert_eq!(root.sequence_id("@test").unwrap(), 1);
	assert_eq!(level1.get::<i32>("@test").unwrap(), 42);
	assert_eq!(level1.sequence_id("@test").unwrap(), 1);
	assert_eq!(level2.get::<i32>("@test").unwrap(), 42);
	assert_eq!(level2.sequence_id("@test").unwrap(), 1);
	assert_eq!(root.get::<i32>("test").unwrap(), 42);
	assert_eq!(root.sequence_id("test").unwrap(), 1);
	assert_eq!(level1.get::<i32>("test").unwrap(), 42);
	assert_eq!(level1.sequence_id("test").unwrap(), 1);
	assert_eq!(level2.get::<i32>("test").unwrap(), 42);
	assert_eq!(level2.sequence_id("test").unwrap(), 1);

	// set 'test' in level2 (should alter '@test')
	assert_eq!(level2.set("test", 44).unwrap(), Some(42));
	assert_eq!(level1.get::<i32>("test").unwrap(), 44);
	assert_eq!(level1.sequence_id("test").unwrap(), 2);
	assert_eq!(level1.get::<i32>("@test").unwrap(), 44);
	assert_eq!(level1.sequence_id("@test").unwrap(), 2);
	assert_eq!(level2.get::<i32>("test").unwrap(), 44);
	assert_eq!(level2.sequence_id("test").unwrap(), 2);
	assert_eq!(level2.get::<i32>("@test").unwrap(), 44);
	assert_eq!(level2.sequence_id("@test").unwrap(), 2);

	// update 'test' from level2 in root
	assert_eq!(level2.set("@test", 22).unwrap(), Some(44));
	assert_eq!(level1.get::<i32>("test").unwrap(), 22);
	assert_eq!(level1.sequence_id("test").unwrap(), 3);
	assert_eq!(level1.get::<i32>("@test").unwrap(), 22);
	assert_eq!(level1.sequence_id("@test").unwrap(), 3);
	assert_eq!(level2.get::<i32>("@test").unwrap(), 22);
	assert_eq!(level2.sequence_id("@test").unwrap(), 3);
	assert_eq!(level2.get::<i32>("test").unwrap(), 22);
	assert_eq!(level2.sequence_id("test").unwrap(), 3);

	// delete 'test' in level2
	assert_eq!(level2.delete::<i32>("test").unwrap(), 22);
	assert!(!root.contains("test"));
	assert!(!level1.contains("test"));
	assert!(!level2.contains("test"));
}
