// Copyright © 2025 Stephan Kunz
//! Itegration tests for [`Remappings`].

#![allow(unused)]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use databoard::{Databoard, Remappings};

#[test]
fn usage() {
	let mut remappings = Remappings::default();
	assert!(remappings.find("remapped").is_none());

	remappings.add("remapped", "test").unwrap();
	assert!(remappings.add("remapped", "test").is_err());
	assert_eq!(remappings.find("remapped").unwrap().as_ref(), "test");

	remappings.overwrite("remapped", "overwritten");
	assert_eq!(remappings.find("remapped").unwrap().as_ref(), "overwritten");

	remappings.overwrite("remapped2", "test");
	assert_eq!(remappings.find("remapped2").unwrap().as_ref(), "test");

	assert!(remappings.find("not_remapped").is_none());

	remappings.shrink_to_fit();
	assert_eq!(remappings.find("remapped").unwrap().as_ref(), "overwritten");
	assert_eq!(remappings.find("remapped2").unwrap().as_ref(), "test");
}
