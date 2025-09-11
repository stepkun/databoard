// Copyright © 2025 Stephan Kunz
//! Implementation of the [`DataboardData`].

#![allow(dead_code, unused)]

use crate::{
	ConstString, Error,
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

/// Struct that holds all [`Databoard`] data.
#[derive(Default)]
pub struct Database {
	storage: BTreeMap<ConstString, EntryPtr>,
}

impl Database {
	pub fn contains(&self, key: &str) -> bool {
		self.storage.contains_key(key)
	}

	pub fn create<T: Send + Sync + 'static>(&mut self, key: impl Into<ConstString>, value: T) -> Result<()> {
		let entry = EntryPtr::new(value);
		if self.storage.insert(key.into(), entry).is_some() {
			return Err(Error::Unexpected(file!().into(), line!()));
		}
		Ok(())
	}

	pub fn delete<T: Send + Sync + 'static>(&mut self, key: &str) -> Result<T> {
		// check type
		if let Some(entry) = self.storage.get(key) {
			let en = &*entry.0.read().data;
			if en.downcast_ref::<T>().is_none() {
				return Err(Error::WrongType { key: key.into() });
			}
		} else {
			return Err(Error::NotFound { key: key.into() });
		}
		if let Some(old) = self.storage.remove(key) {
			let en = old.0.into_inner().data;
			if let Ok(value) = en.downcast::<T>() {
				return Ok(*value);
			}
		}

		// We should never reach this!
		Err(Error::Unexpected(file!().into(), line!()))
	}

	pub fn update<T: Clone + Send + Sync + 'static>(&self, key: &str, value: T) -> Result<T> {
		self.storage.get(key).map_or_else(
			|| Err(Error::NotFound { key: key.into() }),
			|mut entry| {
				let en = &mut *entry.0.write();
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

	pub fn read<T: Clone + Send + Sync + 'static>(&self, key: &str) -> Result<T> {
		self.storage.get(key).map_or_else(
			|| Err(Error::NotFound { key: key.into() }),
			|entry| {
				let en = &*entry.0.read().data;
				let t = en.downcast_ref::<T>();
				t.cloned()
					.map_or_else(|| Err(Error::WrongType { key: key.into() }), |v| Ok(v))
			},
		)
	}

	pub fn sequence_id(&self, key: &str) -> Result<usize> {
		self.storage.get(key).map_or_else(
			|| Err(Error::NotFound { key: key.into() }),
			|entry| Ok(entry.read().sequence_id),
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
