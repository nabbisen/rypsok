//! Standard-library locks that survive a poisoned state (RFC 020, handoff
//! 020j, NET-13).
//!
//! A `std::sync` lock is poisoned when a thread panics while holding it.
//! Calling `.expect(..)` on the result would make every later use panic
//! too, so one panic would end every call that touches the lock. These
//! helpers take the guard out of the poison error instead.
//!
//! This is sound for the locks of this crate and only for them: the
//! critical sections insert into maps, push to queues, read or set an
//! `Instant`, or use saturating arithmetic. None of them can panic, and
//! every state they leave is valid after each statement, so the data behind
//! a poisoned lock is as consistent as behind a healthy one. A new critical
//! section that can leave its data half-updated must not use these helpers.
//!
//! The one deliberate exception is the result cache, which disables itself
//! on a poisoned lock (RFC 014) and handles the error on its own.

use std::sync::{Mutex, MutexGuard, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Locks `mutex`, recovering the guard if the lock is poisoned.
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Takes a read guard of `lock`, recovering it if the lock is poisoned.
pub(crate) fn read<T>(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(PoisonError::into_inner)
}

/// Takes a write guard of `lock`, recovering it if the lock is poisoned.
pub(crate) fn write<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests;
