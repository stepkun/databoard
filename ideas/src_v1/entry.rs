// Copyright © 2025 Stephan Kunz
//! Implementation of the entry for a [`Databoard`](crate::databoard::Databoard).

#![allow(dead_code, unused)]

use crate::{error::Result, remappings::Remappings};
use alloc::{borrow::ToOwned, boxed::Box, collections::btree_map::BTreeMap, string::String, sync::Arc};
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
	/// succeeds, wraps the value and lock into [`EntryGuardInner`].
	pub fn new(entry: EntryPtr) -> Option<Self> {
		// Check if the inner value can be downcasted directly to `T`
		let is_valid = entry.read().downcast_ref::<T>().is_some();
		if is_valid {
			let guard: RwLockWriteGuard<'static, EntryData> = entry.write();
			let inner = EntryGuardInner::new(entry, |guard| entry.write());
			Some(Self(inner))
		} else {
			None
		}
	}
}

impl<T: 'static> Deref for EntryGuard<T> {
	type Target = T;

	#[allow(unsafe_code)]
	fn deref(&self) -> &Self::Target {
		unsafe { &*self.0.ptr }
	}
}

impl<T: 'static> DerefMut for EntryGuard<T> {
	#[allow(unsafe_code)]
	fn deref_mut(&mut self) -> &mut Self::Target {
		// remember modification
		self.0.modified = true;
		unsafe { &mut *self.0.ptr }
	}
}

/// Self-referencing struct that holds
/// - an [`EntryPtr`],
/// - the locked `RwLockWriteGuard` around the [`EntryData`],
/// - a reference to downcasted `T` borrowed from the `RwLockWriteGuard`.
struct EntryGuardInner<T: 'static> {
	entry: EntryPtr,
	guard: RwLockWriteGuard<'static, EntryData>,
	ptr: *mut T,
	modified: bool,
}

impl<T> Drop for EntryGuardInner<T> {
	fn drop(&mut self) {
		if self.modified {
			self.guard.sequence_id += 1;
		}
	}
}

impl<T: 'static> EntryGuardInner<T> {
	#[allow(unsafe_code)]
	#[allow(clippy::coerce_container_to_any)]
	#[allow(clippy::expect_used)]
	pub fn new(entry: EntryPtr, guard: RwLockWriteGuard<'static, EntryData>) -> Self {
		let ptr = {
			// we know this pointer is valid since the guard owns the EntryPtr
			let mut x = &mut entry.write().data;
			let t = x
				.downcast_mut::<T>()
				.expect("this should not happen");

			let ptr: *mut T = unsafe { t };
			ptr
		};

		Self {
			entry,
			guard,
			ptr,
			modified: false,
		}
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
