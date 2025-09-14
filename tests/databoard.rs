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
	assert!(!databoard.contains_key("test"));
	assert!(databoard.get::<i32>("test").is_err());
	assert!(databoard.get::<String>("test").is_err());

	let old = databoard.set::<i32>("test", 42).unwrap();
	assert_eq!(old, None);

	assert!(databoard.contains_key("test"));
	assert!(databoard.contains::<i32>("test").unwrap());
	assert!(databoard.contains::<String>("test").is_err());
	assert_eq!(databoard.sequence_id("test").unwrap(), 1);
	assert_eq!(databoard.get::<i32>("test").unwrap(), 42);
	assert!(databoard.get::<String>("test").is_err());

	assert!(
		databoard
			.set::<String>("test", "fail".into())
			.is_err()
	);

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
	// access from root
	assert!(root.contains_key("test"));
	assert!(root.contains_key("@test"));
	assert!(root.contains::<String>("test").is_err());
	assert!(root.contains::<String>("@test").is_err());
	assert_eq!(root.get::<i32>("test").unwrap(), 42);
	assert_eq!(root.get::<i32>("@test").unwrap(), 42);
	assert!(root.get::<String>("test").is_err());
	assert!(root.get::<String>("@test").is_err());
	assert_eq!(root.sequence_id("test").unwrap(), 1);
	assert_eq!(root.sequence_id("@test").unwrap(), 1);
	// access from level1
	assert!(!level1.contains_key("test"));
	assert!(level1.contains_key("@test"));
	assert_eq!(level1.get::<i32>("@test").unwrap(), 42);
	assert!(level1.get::<String>("@test").is_err());
	assert_eq!(level1.sequence_id("@test").unwrap(), 1);
	// access from level2
	assert!(!level2.contains_key("test"));
	assert_eq!(level2.get::<i32>("@test").unwrap(), 42);
	assert!(level2.get::<String>("@test").is_err());
	assert_eq!(level2.sequence_id("@test").unwrap(), 1);

	// set 'test' in level2
	assert_eq!(level2.set("test", 44).unwrap(), None);
	assert!(!level1.contains_key("test"));
	assert!(level2.contains_key("test"));
	assert!(level2.contains::<i32>("test").unwrap());
	assert!(level2.contains::<String>("test").is_err());
	assert_eq!(level2.get::<i32>("test").unwrap(), 44);
	assert_eq!(level2.sequence_id("test").unwrap(), 1);
	assert_eq!(level2.get::<i32>("@test").unwrap(), 42);
	assert_eq!(level2.sequence_id("@test").unwrap(), 1);

	// update 'test' from level2 in root
	assert_eq!(level2.set("@test", 24).unwrap(), Some(42));
	assert!(!level1.contains_key("test"));
	assert_eq!(level2.get::<i32>("@test").unwrap(), 24);
	assert_eq!(level2.sequence_id("@test").unwrap(), 2);
	assert_eq!(level2.get::<i32>("test").unwrap(), 44);
	assert_eq!(level2.sequence_id("test").unwrap(), 1);

	// update 'test' in level2
	assert_eq!(level2.set("test", 22).unwrap(), Some(44));
	assert!(!level1.contains_key("test"));
	assert_eq!(level2.get::<i32>("@test").unwrap(), 24);
	assert_eq!(level2.sequence_id("@test").unwrap(), 2);
	assert_eq!(level2.get::<i32>("test").unwrap(), 22);
	assert_eq!(level2.sequence_id("test").unwrap(), 2);

	// delete 'test'
	assert_eq!(level2.delete::<i32>("test").unwrap(), 22);
	assert_eq!(level2.delete::<i32>("@test").unwrap(), 24);
	assert!(!root.contains_key("test"));
	assert!(!level1.contains_key("test"));
	assert!(!level2.contains_key("test"));
}

#[test]
fn root_access_auto_remapping() {
	let root = Databoard::new();
	let level1 = Databoard::with_parent(root.clone());
	let level2 = Databoard::with_parent(level1.clone());

	// set 'test' from level2 in root
	assert_eq!(level2.set("@test", 42).unwrap(), None);
	// access from root
	assert!(root.contains_key("test"));
	assert!(root.contains::<i32>("test").unwrap());
	assert!(root.contains::<String>("test").is_err());
	assert_eq!(root.get::<i32>("@test").unwrap(), 42);
	assert_eq!(root.get::<i32>("test").unwrap(), 42);
	assert!(root.get::<String>("@test").is_err());
	assert!(root.get::<String>("test").is_err());
	assert_eq!(root.sequence_id("@test").unwrap(), 1);
	assert_eq!(root.sequence_id("test").unwrap(), 1);
	// access from level1
	assert!(level1.contains_key("test"));
	assert!(level1.contains::<i32>("test").unwrap());
	assert!(level1.contains::<String>("test").is_err());
	assert_eq!(level1.get::<i32>("@test").unwrap(), 42);
	assert_eq!(level1.get::<i32>("test").unwrap(), 42);
	assert!(level1.get::<String>("@test").is_err());
	assert!(level1.get::<String>("test").is_err());
	assert_eq!(level1.sequence_id("@test").unwrap(), 1);
	assert_eq!(level1.sequence_id("test").unwrap(), 1);
	// access from level2
	assert!(level2.contains_key("test"));
	assert!(level2.contains::<i32>("test").unwrap());
	assert!(level2.contains::<String>("test").is_err());
	assert_eq!(level2.get::<i32>("@test").unwrap(), 42);
	assert_eq!(level2.get::<i32>("test").unwrap(), 42);
	assert!(level2.get::<String>("@test").is_err());
	assert!(level2.get::<String>("test").is_err());
	assert_eq!(level2.sequence_id("@test").unwrap(), 1);
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
	assert!(!root.contains_key("test"));
	assert!(!level1.contains_key("test"));
	assert!(!level2.contains_key("test"));
}

#[test]
fn auto_remapping() {
	let root = Databoard::new();
	let level1 = Databoard::with_parent(root.clone());
	let level2 = Databoard::with_parent(level1.clone());

	// set 'test' in root
	assert_eq!(root.set("test", 40).unwrap(), None);
	// set 'test1' in level1
	assert_eq!(level1.set("test1", 41).unwrap(), None);
	// set 'test2' in level2
	assert_eq!(level2.set("test2", 42).unwrap(), None);
	assert_eq!(root.get::<i32>("test").unwrap(), 40);
	assert_eq!(level1.get::<i32>("test").unwrap(), 40);
	assert_eq!(level2.get::<i32>("test").unwrap(), 40);
	assert!(root.get::<i32>("test1").is_err());
	assert_eq!(level1.get::<i32>("test1").unwrap(), 41);
	assert_eq!(level2.get::<i32>("test1").unwrap(), 41);
	assert!(root.get::<i32>("test2").is_err());
	assert!(level1.get::<i32>("test2").is_err());
	assert_eq!(level2.get::<i32>("test2").unwrap(), 42);

	// set 'test' in level1
	assert_eq!(level1.set("test", 41).unwrap(), Some(40));
	// set 'test' in level2
	assert_eq!(level1.set("test", 42).unwrap(), Some(41));

	assert_eq!(root.sequence_id("test").unwrap(), 3);
	assert_eq!(level1.sequence_id("test").unwrap(), 3);
	assert_eq!(level2.sequence_id("test").unwrap(), 3);
	assert_eq!(level1.sequence_id("test1").unwrap(), 1);
	assert_eq!(level2.sequence_id("test2").unwrap(), 1);

	assert_eq!(root.delete::<i32>("test").unwrap(), 42);
	assert!(!root.contains_key("test"));
	assert_eq!(level1.delete::<i32>("test1").unwrap(), 41);
	assert!(!level1.contains_key("test1"));
	assert_eq!(level2.delete::<i32>("test2").unwrap(), 42);
	assert!(!level2.contains_key("test"));
}

#[test]
fn manual_remapping() {
	let root = Databoard::new();
	let mut remappings = Remappings::default();
	remappings.add("test", "test");
	remappings.add("test1", "test");
	let level1 = Databoard::with(Some(root.clone()), Some(remappings), false);
	let mut remappings = Remappings::default();
	remappings.add("test", "test");
	remappings.add("test1", "test1");
	remappings.add("test2", "test");
	remappings.add("testX", "test1");
	let level2 = Databoard::with(Some(level1.clone()), Some(remappings), false);

	// set 'test' in level2
	assert_eq!(level2.set("test", 40).unwrap(), None);
	assert!(level2.contains_key("test"));
	assert!(level2.contains_key("test1"));
	assert!(level2.contains_key("test2"));
	assert!(level2.contains_key("testX"));
	assert!(level1.contains_key("test"));
	assert!(level1.contains_key("test1"));
	assert!(!level1.contains_key("test2"));
	assert!(!level1.contains_key("testX"));
	assert!(root.contains_key("test"));
	assert!(!root.contains_key("test1"));
	assert!(!root.contains_key("test2"));
	assert!(!root.contains_key("testX"));
	// set 'test1' in level2
	assert_eq!(level2.set("test1", 41).unwrap(), Some(40));
	assert!(level2.contains_key("test1"));
	assert!(level1.contains_key("test1"));
	assert!(!root.contains_key("test1"));
	// set 'test2' in level2
	assert_eq!(level2.set("test2", 42).unwrap(), Some(41));
	assert!(level2.contains_key("test2"));
	// set 'testX' in level2
	assert_eq!(level2.set("testX", 44).unwrap(), Some(42));
	assert!(level2.contains_key("testX"));

	assert_eq!(root.get::<i32>("test").unwrap(), 44);
	assert_eq!(level1.get::<i32>("test").unwrap(), 44);
	assert_eq!(level1.get::<i32>("test1").unwrap(), 44);
	assert_eq!(level2.get::<i32>("test").unwrap(), 44);
	assert_eq!(level2.get::<i32>("test1").unwrap(), 44);
	assert_eq!(level2.get::<i32>("test2").unwrap(), 44);
	assert_eq!(level2.get::<i32>("testX").unwrap(), 44);

	assert_eq!(root.sequence_id("test").unwrap(), 4);
	assert_eq!(level1.sequence_id("test").unwrap(), 4);
	assert_eq!(level2.sequence_id("test").unwrap(), 4);

	assert_eq!(level2.delete::<i32>("test2").unwrap(), 44);
	assert!(!root.contains_key("test"));
	assert!(!level1.contains_key("test"));
	assert!(!level1.contains_key("test1"));
	assert!(!level2.contains_key("test"));
	assert!(!level2.contains_key("test1"));
	assert!(!level2.contains_key("test2"));
}

#[test]
fn mixed_remapping() {
	let root = Databoard::new();
	let mut remappings = Remappings::default();
	remappings.add("manual1", "manual");
	let level1 = Databoard::with(Some(root.clone()), Some(remappings), true);
	let mut remappings = Remappings::default();
	remappings.add("manual2", "manual1");
	let level2 = Databoard::with(Some(level1.clone()), Some(remappings), true);

	// set 'test' in root
	assert_eq!(root.set("test", 42).unwrap(), None);
	assert_eq!(root.get::<i32>("test").unwrap(), 42);
	assert_eq!(level1.get::<i32>("test").unwrap(), 42);
	assert_eq!(level2.get::<i32>("test").unwrap(), 42);

	assert!(root.contains_key("test"));
	assert!(level1.contains_key("test"));
	assert!(level2.contains_key("test"));

	assert_eq!(level2.sequence_id("test").unwrap(), 1);
	assert_eq!(level2.delete::<i32>("test").unwrap(), 42);
	assert!(!root.contains_key("test"));
	assert!(!level1.contains_key("test"));
	assert!(!level2.contains_key("test"));

	// set 'manual2' in level2
	assert_eq!(level2.set("manual2", 24).unwrap(), None);
	assert_eq!(root.get::<i32>("manual").unwrap(), 24);
	assert_eq!(level1.get::<i32>("manual1").unwrap(), 24);
	assert_eq!(level2.get::<i32>("manual2").unwrap(), 24);

	assert!(root.contains_key("manual"));
	assert!(level1.contains_key("manual1"));
	assert!(level2.contains_key("manual2"));

	assert_eq!(level2.sequence_id("manual").unwrap(), 1);
	assert_eq!(level2.delete::<i32>("manual2").unwrap(), 24);
	assert!(!root.contains_key("manual"));
	assert!(!level1.contains_key("manual1"));
	assert!(!level2.contains_key("manual2"));
}

#[test]
fn referencing() {
	let databoard = Databoard::new();
	assert!(databoard.get_mut_ref::<i32>("test)").is_err());
	assert!(databoard.get_mut_ref::<String>("test)").is_err());

	let old = databoard.set::<i32>("test", 42).unwrap();
	assert_eq!(old, None);
	assert!(databoard.get_mut_ref::<String>("test)").is_err());

	let mut entry = databoard.get_mut_ref::<i32>("test").unwrap();
	// read tests
	assert_eq!(*entry, 42);
	// write tests
	*entry = 22;
	*entry += 4;
	*entry -= 2;
	assert_eq!(*entry, 24);
	drop(entry);
	assert_eq!(databoard.get::<i32>("test").unwrap(), 24);

	assert_eq!(databoard.delete::<i32>("test").unwrap(), 24);
	assert!(!databoard.contains_key("test"));
}
