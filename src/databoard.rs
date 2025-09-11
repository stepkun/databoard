// Copyright © 2025 Stephan Kunz
//! Implementation of the [`Databoard`].

#![allow(dead_code, unused)]

use crate::{
	ConstString, Error,
	database::Database,
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
	database: Arc<RwLock<Database>>,
	/// An optional reference to a parent `Databoard`.
	parent: Option<Arc<Databoard>>,
	/// Manual remapping rules from this `Databoard` to the parent.
	remappings: Remappings,
	/// Whether to use automatic remapping to parents content.
	autoremap: bool,
}

impl Databoard {
	/// Creates a [`DataboardPtr`] to a new `Databoard`.
	#[must_use]
	pub fn new() -> DataboardPtr {
		Arc::new(Self {
			database: Arc::new(RwLock::new(Database::default())),
			parent: None,
			remappings: Remappings::default(),
			autoremap: false,
		})
	}

	/// Creates a [`DataboardPtr`] to a new `Databoard` with given parameters.
	pub fn with(parent: Option<DataboardPtr>, remappings: Option<Remappings>, autoremap: bool) -> DataboardPtr {
		let remappings = remappings.map_or_else(Remappings::default, |remappings| remappings);
		let database = Arc::new(RwLock::new(Database::default()));
		Arc::new(Self {
			database,
			parent,
			remappings,
			autoremap,
		})
	}

	/// Creates a [`DataboardPtr`] to a new `Databoard` with a parent.
	#[must_use]
	pub fn with_parent(parent: DataboardPtr) -> DataboardPtr {
		let database = Arc::new(RwLock::new(Database::default()));
		Arc::new(Self {
			database,
			parent: Some(parent),
			remappings: Remappings::default(),
			autoremap: true,
		})
	}

	/// Returns `true` if a certain `key` is available, otherwise `false`.
	#[must_use]
	pub fn contains(&self, key: &str) -> bool {
		// if it is a key starting with an '@' redirect to root board
		if let Some(key_stripped) = key.strip_prefix('@') {
			return self.root().contains(key_stripped);
		}

		// look in database
		if self.database.read().contains(key) {
			return true;
		}

		// autoremapping?
		if self.autoremap
			&& let Some(parent) = &self.parent
		{
			return parent.contains(key);
		}

		false
	}

	/// Returns a value of type `T` stored under `key` and deletes it from storage.
	/// # Errors
	/// - if `key` is not contained
	/// - if the entry has not the expected type `T`
	pub fn delete<T: Send + Sync + 'static>(&self, key: &str) -> Result<T> {
		// if it is a key starting with an '@' redirect to root board
		if let Some(key_stripped) = key.strip_prefix('@') {
			return self.root().delete(key_stripped);
		}

		// look in database
		if self.database.read().contains(key) {
			return self.database.write().delete(key);
		}

		// Try to find in parent hierarchy.
		let (parent_key, has_remapping, autoremap) = self.remapping_info(key);
		if (has_remapping || autoremap)
			&& let Some(parent) = &self.parent
			&& parent.contains(&parent_key)
		{
			return parent.delete(key);
		}

		Err(Error::NotFound { key: key.into() })
	}

	/// Returns a copy of the raw [`Entry`] stored under `key`.
	#[must_use]
	pub fn entry(&self, key: &str) -> Option<EntryData> {
		// if it is a key starting with an '@' redirect to root board
		if let Some(key_stripped) = key.strip_prefix('@') {
			return self.root().entry(key_stripped);
		}

		todo!()
	}

	/// Returns a copy of the value of type `T` stored under `key`.
	/// # Errors
	/// - if `key` is not contained
	/// - if the entry has not the expected type `T`
	pub fn get<T: Clone + Send + Sync + 'static>(&self, key: &str) -> Result<T> {
		// if it is a key starting with an '@' redirect to root board
		if let Some(key_stripped) = key.strip_prefix('@') {
			return self.root().get(key_stripped);
		}

		// look in database
		if let Ok(value) = self.database.read().read(key) {
			return Ok(value);
		}

		// autoremapping?
		if self.autoremap
			&& let Some(parent) = &self.parent
		{
			return parent.get(key);
		}

		Err(Error::NotFound { key: key.into() })
	}

	/// Returns a read/write guard to the `T` for the `key`.
	#[must_use]
	pub fn guard<T>(&self, key: &str) -> EntryGuard<T> {
		// if it is a key starting with an '@' redirect to root board
		if let Some(key_stripped) = key.strip_prefix('@') {
			return self.root().guard(key_stripped);
		}
		todo!()
	}

	/// Returns to the root [`Databoard`] of the hierarchy.
	fn root(&self) -> &Self {
		self.parent
			.as_ref()
			.map_or(self, |board| board.root())
	}

	/// Stores a value of type `T` under `key` and returns an eventually existing value.
	/// # Errors
	/// - if `key` already exists with a different type
	pub fn set<T: Clone + Send + Sync + 'static>(&self, key: &str, value: T) -> Result<Option<T>> {
		// if it is a key starting with an '@' redirect to root board
		if let Some(key_stripped) = key.strip_prefix('@') {
			return self.root().set(key_stripped, value);
		}

		// first look in own database
		if self.database.read().contains(key) {
			let old = self.database.read().update(key, value)?;
			return Ok(Some(old));
		}

		// Try to find in parent hierarchy.
		let (parent_key, has_remapping, autoremap) = self.remapping_info(key);
		if (has_remapping || autoremap)
			&& let Some(parent) = &self.parent
			&& parent.contains(&parent_key)
		{
			return parent.set(&parent_key, value);
		}

		// If it is not remapped anywhere in hierarchy, create it in current `Blackboard`
		self.database.write().create(key, value)?;
		Ok(None)
	}

	/// Returns the sequence id of an entry.
	/// The sequence id is increased at every change of an entry and will wrap around.
	/// # Errors
	/// - if `key` is not contained
	pub fn sequence_id(&self, key: &str) -> Result<usize> {
		// if it is a key starting with an '@' redirect to root board
		if let Some(key_stripped) = key.strip_prefix('@') {
			return self.root().sequence_id(key_stripped);
		}

		// look in database
		if let Ok(value) = self.database.read().sequence_id(key) {
			return Ok(value);
		}

		// autoremapping?
		if self.autoremap
			&& let Some(parent) = &self.parent
		{
			return parent.sequence_id(key);
		}

		Err(Error::NotFound { key: key.into() })
	}

	/// Read needed remapping information to parent.
	fn remapping_info(&self, key: &str) -> (ConstString, bool, bool) {
		let (remapped_key, has_remapping) = self
			.remappings
			.find(key)
			.map_or_else(|| (key.into(), false), |remapped| (remapped, true));

		(remapped_key, has_remapping, self.autoremap)
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
