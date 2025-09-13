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
use spin::RwLock;

/// A pointer to the [`EntryData`].
#[repr(transparent)]
#[derive(Clone)]
pub struct EntryPtr(pub(crate) Arc<RwLock<EntryData>>);

impl Deref for EntryPtr {
	type Target = Arc<RwLock<EntryData>>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl DerefMut for EntryPtr {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl EntryPtr {
	pub(crate) fn new<T: Send + Sync + 'static>(value: T) -> Self {
		let data = EntryData {
			data: Box::new(value),
			// start sequence with 1
			sequence_id: usize::MIN + 1,
		};
		Self(Arc::new(RwLock::new(data)))
	}
}
/// The data behind an [`EntryPtr`].
pub struct EntryData {
	pub(crate) data: Box<dyn Any + Send + Sync>,
	pub(crate) sequence_id: usize,
}

/// Guard for a reference to a[`Databoard`](crate::databoard::Databoard) entry.
/// Implements [`Deref`] and [`DerefMut`], providing read and write access to the `T`.
pub type EntryGuard<T> = RwLock<EntryGuardInner<T>>;

/// Inner data for the [`EntryGuard`].
/// Implements [`Deref`] and [`DerefMut`], providing read and write access to the `T`.
pub struct EntryGuardInner<T> {
	entry: EntryPtr,
	modified: bool,
	ptr: *mut T,
}

impl<T> Deref for EntryGuardInner<T> {
	type Target = T;

	#[allow(unsafe_code)]
	fn deref(&self) -> &Self::Target {
		let t = self.entry.0.read();
		unsafe { &*self.ptr }
	}
}

impl<T> DerefMut for EntryGuardInner<T> {
	#[allow(unsafe_code)]
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.modified = true;
		unsafe { &mut *self.ptr }
	}
}

impl<T> Drop for EntryGuardInner<T> {
	fn drop(&mut self) {
		if self.modified {
			self.entry.0.write().sequence_id += 1;
		}
	}
}

impl<T: 'static> EntryGuardInner<T> {
	#[allow(unsafe_code)]
	#[allow(clippy::coerce_container_to_any)]
	#[allow(clippy::expect_used)]
	pub fn new(entry: EntryPtr) -> Self {
		let ptr = {
			// we know this pointer is valid since the guard owns the EntryPtr
			let mut x = &mut entry.0.write().data;
			let t = x
				.downcast_mut::<T>()
				.expect("this should not happen");

			let ptr: *mut T = unsafe { t };
			ptr
		};

		Self {
			entry,
			modified: false,
			ptr,
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
		is_normal::<EntryPtr>();
		is_normal::<EntryData>();
	}
}
