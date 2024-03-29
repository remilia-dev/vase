// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.

// NOTE: Some other crate's sync utilities are exported here to allow for easy changes and to provide a shortcut.

// Standard library re-exports
#[doc(hidden)]
pub use std::sync::atomic::{
    AtomicBool,
    AtomicPtr,
    AtomicU8,
    AtomicU32,
    AtomicUsize,
    Ordering,
};
#[doc(hidden)]
pub use std::sync::Arc;

// Library re-exports
#[doc(hidden)]
pub use parking_lot::{
    Condvar,
    Mutex,
    RwLock,
    RwLockUpgradableReadGuard,
};

// Re-exports of this crate's sync utilities
// The self:: prefix is to prevent rustfmt from combining them with the re-exports.
#[cfg(feature = "multithreading")]
pub use self::work_queue::WorkQueue;
pub use self::{
    atomic_arc::AtomicArc,
    atomic_box::AtomicBox,
    once_array::OnceArray,
};

mod atomic_arc;
mod atomic_box;
mod once_array;
#[cfg(feature = "multithreading")]
mod work_queue;
