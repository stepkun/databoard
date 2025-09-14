// Copyright © 2025 Stephan Kunz
//! Implementation of the entry for a [`Databoard`](crate::databoard::Databoard).

#![allow(dead_code, unused)]

use crate::{error::Result, remappings::Remappings};
use alloc::{boxed::Box, collections::btree_map::BTreeMap, string::String, sync::Arc};
use core::{
	any::Any,
	marker::PhantomData,
	ops::{Deref, DerefMut},
};
use ouroboros::self_referencing;
use spin::{RwLock, RwLockReadGuard, RwLockWriteGuard};

/// The data stored in a [`Databoard`](crate::databoard::Databoard) entry.
pub struct EntryData {
	pub(crate) data: Box<dyn Any + Send + Sync>,
	pub(crate) sequence_id: usize,
}

impl Deref for EntryData {
	type Target = Box<dyn Any + Send + Sync>;

	fn deref(&self) -> &Self::Target {
		&self.data
	}
}

impl DerefMut for EntryData {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.data
	}
}

impl EntryData {
	/// Creates a new `EntryData`.
	pub fn new<T: 'static + Send + Sync>(value: T) -> Self {
		Self {
			data: Box::new(value),
			sequence_id: usize::MIN + 1,
		}
	}
}

/// Convenience type for the Arc around the [`EntryData`]
pub type EntryPtr = Arc<RwLock<EntryData>>;

/// Self-referencing struct that holds
/// - an [`EntryPtr`],
/// - the locked `RwLockWriteGuard` around the [`EntryData`],
/// - a reference to downcasted `T` borrowed from the `RwLockWriteGuard`.
struct EntryGuardInner<T: 'static> {
	entry: EntryPtr,
	guard: RwLockWriteGuard<'this, EntryData>,
	ptr: *mut T,
}

/// A reference to the locked [`EntryData`].
#[repr(transparent)]
#[derive(Clone)]
pub struct EntryRef(pub(crate) EntryPtr);

impl EntryRef {
	pub(crate) fn new<T: Send + Sync + 'static>(value: T) -> Self {
		let data = EntryData {
			data: Box::new(value),
			// start sequence with 1
			sequence_id: usize::MIN + 1,
		};
		Self(Arc::new(RwLock::new(data)))
	}
}

/// Write-Locked entry.
/// Until this value is dropped, the lock is held on the entry.
///
/// Implements [`Deref`], providing access to the locked `T`.
pub struct EntryGuard<T: 'static>(EntryGuardInner<T>);

impl<T: 'static> EntryGuard<T> {
	/// Attempts to downcast the `Box<dyn Any + Send>` to `T`. If downcasting
	/// succeeds, wraps the value and lock into [`EntryGuard`].
	pub fn create(entry: EntryPtr) -> Option<Self> {
		// Check if the inner value can be downcasted directly to `T`
		let is_valid = entry.read().downcast_ref::<T>().is_some();

		if is_valid {
			let inner = EntryGuardInner::new(
				entry,
				|entry| entry.write(),
				|guard| {
					guard
						.downcast_ref::<T>()
						.expect("downcasting should be possible")
				},
			);

			Some(Self(inner))
		} else {
			None
		}
	}
}

impl<T: 'static> Deref for EntryGuard<T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.0.borrow_value()
	}
}

impl<T: 'static> DerefMut for EntryGuard<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		// unsafe { &mut *self.0.ptr }
		todo!() //self.0.borrow_ptr()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	// check, that the auto traits are available
	const fn is_normal<T: Sized + Send + Sync>() {}

	#[test]
	const fn normal_types() {
		is_normal::<EntryRef>();
		is_normal::<EntryData>();
	}
}
