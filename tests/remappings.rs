// Copyright © 2025 Stephan Kunz
//! Integration tests for [`Remappings`].

#![allow(unused)]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use databoard::{
	Databoard, Remappings, check_board_pointer, check_local_key, check_local_pointer, check_top_level_key,
	check_top_level_pointer, is_board_pointer, is_const_assignment, is_local_pointer, is_top_level_pointer,
	strip_board_pointer, strip_local_pointer, strip_top_level_pointer,
};

#[test]
fn const_assignment_helpers() {
	assert!(is_const_assignment("key"));
	assert!(!is_const_assignment("{key}"));
	assert!(!is_const_assignment("key}"));
	assert!(!is_const_assignment("{key"));
}

#[test]
fn key_helpers() {
	assert_eq!(check_local_key("_key"), Ok("key".into()));
	assert_eq!(check_local_key("@key"), Err("@key"));
	assert_eq!(check_local_key("key"), Err("key"));
	assert_eq!(check_local_key("{_key}"), Err("{_key}"));

	assert_eq!(check_top_level_key("@key"), Ok("key".into()));
	assert_eq!(check_top_level_key("_key"), Err("_key"));
	assert_eq!(check_top_level_key("key"), Err("key"));
	assert_eq!(check_top_level_key("{@key}"), Err("{@key}"));
}

#[test]
fn board_pointer_helpers() {
	assert!(is_board_pointer("{key}"));
	assert!(is_board_pointer("{_key}"));
	assert!(is_board_pointer("{@key}"));
	assert!(!is_board_pointer("key"));
	assert!(!is_board_pointer("key}"));
	assert!(!is_board_pointer("{key"));

	assert_eq!(strip_board_pointer("{key}"), Some("key".into()));
	assert_eq!(strip_board_pointer("{_key}"), Some("_key".into()));
	assert_eq!(strip_board_pointer("{@key}"), Some("@key".into()));
	assert_eq!(strip_board_pointer("key"), None);
	assert_eq!(strip_board_pointer("key}"), None);
	assert_eq!(strip_board_pointer("{key"), None);

	assert_eq!(check_board_pointer("{key}"), Ok("key".into()));
	assert_eq!(check_board_pointer("{_key}"), Ok("_key".into()));
	assert_eq!(check_board_pointer("{@key}"), Ok("@key".into()));
	assert_eq!(check_board_pointer("key"), Err("key"));
	assert_eq!(check_board_pointer("key}"), Err("key}"));
	assert_eq!(check_board_pointer("{key"), Err("{key"));
}

#[test]
fn local_pointer_helpers() {
	assert!(is_local_pointer("{_key}"));
	assert!(!is_local_pointer("{@key}"));
	assert!(!is_local_pointer("{key}"));
	assert!(!is_local_pointer("_key}"));
	assert!(!is_local_pointer("{_key"));
	assert!(!is_local_pointer("_key"));

	assert_eq!(strip_local_pointer("{_key}"), Some("key".into()));
	assert_eq!(strip_local_pointer("{key}"), None);
	assert_eq!(strip_local_pointer("{@key}"), None);
	assert_eq!(strip_local_pointer("key"), None);
	assert_eq!(strip_local_pointer("key}"), None);
	assert_eq!(strip_local_pointer("{key"), None);

	assert_eq!(check_local_pointer("{_key}"), Ok("key".into()));
	assert_eq!(check_local_pointer("{key}"), Err("{key}"));
	assert_eq!(check_local_pointer("{@key}"), Err("{@key}"));
	assert_eq!(check_local_pointer("key"), Err("key"));
	assert_eq!(check_local_pointer("key}"), Err("key}"));
	assert_eq!(check_local_pointer("{key"), Err("{key"));
}

#[test]
fn top_level_pointer_helpers() {
	assert!(is_top_level_pointer("{@key}"));
	assert!(!is_top_level_pointer("{_key}"));
	assert!(!is_top_level_pointer("{key}"));
	assert!(!is_top_level_pointer("@key}"));
	assert!(!is_top_level_pointer("{@key"));
	assert!(!is_top_level_pointer("@key"));

	assert_eq!(strip_top_level_pointer("{@key}"), Some("key".into()));
	assert_eq!(strip_top_level_pointer("{key}"), None);
	assert_eq!(strip_top_level_pointer("{_key}"), None);
	assert_eq!(strip_top_level_pointer("key"), None);
	assert_eq!(strip_top_level_pointer("key}"), None);
	assert_eq!(strip_top_level_pointer("{key"), None);

	assert_eq!(check_top_level_pointer("{@key}"), Ok("key".into()));
	assert_eq!(check_top_level_pointer("{key}"), Err("{key}"));
	assert_eq!(check_top_level_pointer("{_key}"), Err("{_key}"));
	assert_eq!(check_top_level_pointer("key"), Err("key"));
	assert_eq!(check_top_level_pointer("key}"), Err("key}"));
	assert_eq!(check_top_level_pointer("{key"), Err("{key"));
}

#[test]
fn usage() {
	let mut remappings = Remappings::default();
	assert!(remappings.find("remapped").is_none());

	remappings.add("remapped", "test").unwrap();
	assert!(remappings.add("remapped", "test").is_err());
	assert!(remappings.find("test").is_none());
	assert_eq!(remappings.find("remapped").unwrap().as_ref(), "test");
	assert_eq!(remappings.remap("test").as_ref(), "test");
	assert_eq!(remappings.remap("remapped").as_ref(), "test");

	remappings.overwrite("remapped", "overwritten");
	assert!(remappings.find("test").is_none());
	assert_eq!(remappings.find("remapped").unwrap().as_ref(), "overwritten");
	assert_eq!(remappings.remap("test").as_ref(), "test");
	assert_eq!(remappings.remap("remapped").as_ref(), "overwritten");

	remappings.overwrite("remapped2", "test");
	assert!(remappings.find("test").is_none());
	assert_eq!(remappings.find("remapped2").unwrap().as_ref(), "test");
	assert_eq!(remappings.remap("test").as_ref(), "test");
	assert_eq!(remappings.remap("remapped").as_ref(), "overwritten");
	assert_eq!(remappings.remap("remapped2").as_ref(), "test");

	assert!(remappings.find("not_remapped").is_none());
	assert_eq!(remappings.remap("not_remapped").as_ref(), "not_remapped");

	remappings.shrink_to_fit();
	assert!(remappings.find("test").is_none());
	assert_eq!(remappings.find("remapped").unwrap().as_ref(), "overwritten");
	assert_eq!(remappings.find("remapped2").unwrap().as_ref(), "test");
	assert_eq!(remappings.remap("test").as_ref(), "test");
	assert_eq!(remappings.remap("remapped").as_ref(), "overwritten");
	assert_eq!(remappings.remap("remapped2").as_ref(), "test");
}
