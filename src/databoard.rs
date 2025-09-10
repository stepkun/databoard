// Copyright © 2025 Stephan Kunz
//! Implementation of the [`Databoard`].

#![allow(dead_code, unused)]

use crate::{
	ConstString, Error,
	databoard_data::DataboardData,
	entry::{EntryData, EntryGuard, EntryPtr},
	error::Result,
	remappings::Remappings,
};
use alloc::{borrow::ToOwned, boxed::Box, collections::btree_map::BTreeMap, string::String, sync::Arc};
use core::{
	any::Any,
	ops::{Deref, DerefMut},
};
use spin::RwLock;

/// Convenience type for a pointer to a [`Databoard`].
pub type DataboardPtr = Arc<Databoard>;

/// A Databoard implements both: a [`Blackboard`] and a [`Datastore`].
pub struct Databoard {
	/// Database of this `Databoard`.
	database: Arc<RwLock<DataboardData>>,
	/// An optional reference to a parent `Databoard`.
	parent: Option<Arc<Databoard>>,
}

impl Databoard {
	/// Creates a [`DataboardPtr`] to a new `Databoard`.
	pub fn new() -> DataboardPtr {
		Arc::new(Self {
			database: Arc::new(RwLock::new(DataboardData::default())),
			parent: None,
		})
	}

	/// Creates a [`DataboardPtr`] to a new `Databoard` with given parameters.
	pub fn with(parent: Option<DataboardPtr>, remappings: Option<Remappings>, autoremap: bool) -> DataboardPtr {
		let remappings = if let Some(remappings) = remappings {
			remappings
		} else {
			Remappings::default()
		};
		let database = Arc::new(RwLock::new(DataboardData::with(remappings, autoremap)));
		Arc::new(Self { database, parent })
	}

	/// Creates a [`DataboardPtr`] to a new `Databoard` with a parent.
	pub fn with_parent(parent: DataboardPtr) -> DataboardPtr {
		let database = Arc::new(RwLock::new(DataboardData::with(Remappings::default(), true)));
		Arc::new(Self {
			database,
			parent: Some(parent),
		})
	}

	/// Returns `true` if a certain `key` is available, otherwise `false`.
	pub fn contains(&self, key: &str) -> bool {
		// look in database
		self.database.read().contains(key)
	}

	/// Returns a value of type `T` stored under `key` and deletes it from storage.
	/// # Errors
	/// - if `key` is not contained
	/// - if the entry has not the expected type `T`
	pub fn delete<T: Send + Sync + 'static>(&self, key: &str) -> Result<T> {
		self.database.write().delete(key)
	}

	/// Returns a copy of the raw [`Entry`] stored under `key`.
	pub fn entry(&self, key: &str) -> Option<EntryData> {
		todo!()
	}

	/// Returns a copy of the value of type `T` stored under `key`.
	/// # Errors
	/// - if `key` is not contained
	/// - if the entry has not the expected type `T`
	pub fn get<T: Clone + Send + Sync + 'static>(&self, key: &str) -> Result<T> {
		self.database.read().read(key)
	}

	/// Returns a read/write guard to the `T` for the `key`.
	pub fn guard<T>(&self, key: &str) -> EntryGuard<T> {
		todo!()
	}

	/// Stores a value of type `T` under `key` and returns an eventually existing value.
	/// # Errors
	/// - if `key` already exists with a different type
	pub fn set<T: Clone + Send + Sync + 'static>(&self, key: &str, value: T) -> Result<Option<T>> {
		if self.contains(key) {
			let old = self.database.read().update(key, value)?;
			Ok(Some(old))
		} else {
			// key is not yet used
			self.database.write().create(key, value)?;
			Ok(None)
		}
	}

	/// Returns the sequence id of an entry.
	/// The sequence id is increased at every change of an entry and will wrap around.
	/// # Errors
	/// - if `key` is not contained
	pub fn sequence_id(&self, key: &str) -> Result<usize> {
		self.database.read().sequence_id(key)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	// check, that the auto traits are available
	const fn is_normal<T: Sized + Send + Sync>() {}

	#[test]
	const fn normal_types() {
		is_normal::<Databoard>();
		is_normal::<DataboardPtr>();
	}
}
