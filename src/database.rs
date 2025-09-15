// Copyright © 2025 Stephan Kunz
//! Implementation of the [`DataboardData`].

#![allow(dead_code, unused)]

use crate::{
	ConstString, Error,
	entry::{EntryData, EntryGuard, EntryGuardInner, EntryPtr},
	error::Result,
	remappings::Remappings,
};
use alloc::{borrow::ToOwned, boxed::Box, collections::btree_map::BTreeMap, string::String, sync::Arc};
use core::{
	any::Any,
	ops::{Deref, DerefMut},
};
use spin::RwLock;

/// Holds all [`Databoard`](crate::databoard::Databoard) data.
#[derive(Default)]
pub struct Database {
	storage: BTreeMap<ConstString, EntryPtr>,
}

impl Database {
	/// Returns `true` if a certain `key` is available, otherwise `false`.
	pub fn contains_key(&self, key: &str) -> bool {
		self.storage.contains_key(key)
	}

	/// Returns  a result of `true` if a certain `key` is available, otherwise a result of `false`.
	/// # Errors
	/// - [`Error::WrongType`] if the entry has not the expected type `T`
	pub fn contains<T: 'static>(&self, key: &str) -> Result<bool> {
		if let Some(entry) = self.storage.get(key) {
			let en = &*entry.read().data;
			if en.downcast_ref::<T>().is_none() {
				return Err(Error::WrongType { key: key.into() });
			}
			return Ok(true);
		}
		Ok(false)
	}

	/// Creates a value of type `T` under `key`.
	/// # Errors
	/// - [`Error::AlreadyExists`] if `key` already exists
	pub fn create<T: Send + Sync + 'static>(&mut self, key: impl Into<ConstString>, value: T) -> Result<()> {
		let key = key.into();
		if self.storage.contains_key(&key) {
			return Err(Error::AlreadyExists { key });
		}

		let entry = Arc::new(RwLock::new(EntryData::new(value)));
		if self.storage.insert(key, entry).is_some() {
			return Err(Error::Unexpected(file!().into(), line!()));
		}
		Ok(())
	}

	/// Returns a value of type `T` stored under `key` and deletes it from storage.
	/// # Errors
	/// - [`Error::NotFound`] if `key` is not contained
	/// - [`Error::WrongType`] if the entry has not the expected type `T`
	pub fn delete<T: Clone + Send + Sync + 'static>(&mut self, key: &str) -> Result<T> {
		// check type
		if let Some(entry) = self.storage.get(key) {
			let en = &*entry.read().data;
			if en.downcast_ref::<T>().is_none() {
				return Err(Error::WrongType { key: key.into() });
			}
		} else {
			return Err(Error::NotFound { key: key.into() });
		}
		if let Some(old) = self.storage.remove(key) {
			let en = &*old.read().data;
			let t = en.downcast_ref::<T>();
			return t
				.cloned()
				.map_or_else(|| Err(Error::WrongType { key: key.into() }), |v| Ok(v));
		}

		// We should never reach this!
		Err(Error::Unexpected(file!().into(), line!()))
	}

	/// Returns a read/write guard to the `T` of the `entry` stored under `key`.
	/// The entry is locked for read & write while this reference is held.
	/// Multiple changes during holding the reference are counted as a single change,
	/// so `sequence_id()`will only increase by 1.
	///
	/// You need to drop the received [`EntryGuard`] before using `delete`, `read`, `update` or `sequence_id`.
	/// # Errors
	/// - [`Error::NotFound`] if `key` is not contained
	/// - [`Error::WrongType`] if the entry has not the expected type `T`
	pub fn get_mut_ref<T: 'static>(&self, key: &str) -> Result<EntryGuard<T>> {
		// @TODO: This is a non-optimal implementation
		if let Some(entry) = self.storage.get(key) {
			// ensure that it is the right type before creating reference
			{
				let en = &*entry.read().data;
				if en.downcast_ref::<T>().is_none() {
					return Err(Error::WrongType { key: key.into() });
				}
			}
			return Ok(EntryGuard::new(entry.clone()));
		}

		Err(Error::NotFound { key: key.into() })
	}

	/// Returns a copy of the value of type `T` stored under `key`.
	/// # Errors
	/// - [`Error::NotFound`] if `key` is not contained
	/// - [`Error::WrongType`] if the entry has not the expected type `T`
	pub fn read<T: Clone + Send + Sync + 'static>(&self, key: &str) -> Result<T> {
		self.storage.get(key).map_or_else(
			|| Err(Error::NotFound { key: key.into() }),
			|entry| {
				let en = &*entry.read().data;
				let t = en.downcast_ref::<T>();
				t.cloned()
					.map_or_else(|| Err(Error::WrongType { key: key.into() }), |v| Ok(v))
			},
		)
	}

	/// Returns the sequence id of an entry.
	/// The sequence id starts with '1' and is increased at every change of an entry.
	/// The sequence wraps around to '1' after reaching [`usize::MAX`] .
	/// # Errors
	/// - [`Error::NotFound`] if `key` is not contained
	pub fn sequence_id(&self, key: &str) -> Result<usize> {
		self.storage.get(key).map_or_else(
			|| Err(Error::NotFound { key: key.into() }),
			|entry| Ok(entry.read().sequence_id),
		)
	}

	/// Updates a value of type `T` stored under `key` and returns the old value.
	/// # Errors
	/// - [`Error::NotFound`] if `key` is not contained
	/// - [`Error::WrongType`] if the entry has not the expected type `T`
	pub fn update<T: Clone + Send + Sync + 'static>(&self, key: &str, value: T) -> Result<T> {
		self.storage.get(key).map_or_else(
			|| Err(Error::NotFound { key: key.into() }),
			|mut entry| {
				let en = &mut *entry.write();
				let t = en.data.downcast_ref::<T>();
				t.cloned().map_or_else(
					|| Err(Error::WrongType { key: key.into() }),
					|v| {
						en.data = Box::new(value);
						if en.sequence_id < usize::MAX {
							en.sequence_id += 1;
						} else {
							en.sequence_id = usize::MIN + 1;
						}
						Ok(v)
					},
				)
			},
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	// check, that the auto traits are available
	const fn is_normal<T: Sized + Send + Sync>() {}

	#[test]
	const fn normal_types() {
		is_normal::<Database>();
	}
}
