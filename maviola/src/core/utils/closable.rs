//! # Abstractions for closable tasks and resources
//!
//! This module provides abstractions for resources which state has to be tracked in distributed
//! computations.
//!
//! There are three level ow resource ownership:
//!
//! * [`Closer`] represents a resource which is closed when its owner goes out of scope (similar to
//!   the regular Rust ownership model). This struct is intentionally not [`Clone`].
//! * [`SharedCloser`] represent a resources with shared owners, that is valid while at least one
//!   of the owners has a copy (similar to [`Arc`]). This struct implements [`Clone`].
//! * [`Closable`] represents a dependent resource or task which is notified when resource is no
//!   longer available. A [`Closable`] is read-only.
//!
//! Each level may be "downgraded" to a lower one. A [`Closer`] can produce a [`SharedCloser`], and
//! both of them can create a [`Closable`].
//!
//! The idea behind this hierarchy is that an expensive resource like a server listener ([`Closer`])
//! may be bounded to several co-dependent resources like connection interfaces exposed to a library
//! client ([`SharedCloser`]). If one of the sides are gone, then all should stop. Furthermore,
//! entities like individual peer connections are purely dependant. Which means, their state does
//! not define the state of the server, but at the same time they have to monitor the global state
//! and stop when the main system is stopped.

use std::mem;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::{atomic, Arc};

/// State of a resource or operation that governed by the main task.
///
/// [`Closer`] is owned by a resource that can be closed. When [`Closer::close`] is called or
/// object goes out of scope, it becomes closed. Owner of [`Closer`] can share a read-only
/// view of the "closed" state to multiple listeners. Read-only access is provided by [`Closable`]
/// which can be obtained by calling [`Closer::as_closable`].
///
/// # Usage
///
/// Use a standalone [`Closer`].
///
///```rust
/// use maviola::core::utils::closable::{Closer, Closable};
///
/// let closer = Closer::new();
/// assert!(!closer.is_closed());
///
/// let closable_1 = closer.as_closable();
/// let closable_2 = closable_1.clone();
///
/// assert!(!closable_1.is_closed());
/// assert!(!closable_2.is_closed());
///
/// drop(closer);
///
/// assert!(closable_1.is_closed());
/// assert!(closable_2.is_closed());
/// ```
///
/// It is possible to "share" [`Closer`] using [`Arc`]. In such case a resource will be closed only
/// once all copies will come out of scope:
///
/// ```rust
/// use std::sync::Arc;
/// use maviola::core::utils::closable::{Closer, Closable};
///
/// let closer_1 = Arc::new(Closer::new());
/// let closer_2 = closer_1.clone();
///
/// let closable_1 = closer_1.as_closable();
/// let closable_2 = closable_1.clone();
///
/// drop(closer_1);
///
/// assert!(!closer_2.is_closed());
/// assert!(!closable_1.is_closed());
/// assert!(!closable_2.is_closed());
///
/// drop(closer_2);
///
/// assert!(closable_1.is_closed());
/// assert!(closable_2.is_closed());
///```
///
#[derive(Debug)]
pub struct Closer(Arc<AtomicBool>);

impl Closer {
    /// Creates an instance of closable.
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    /// Returns an instance of [`Closable`], a read-only accessor to the internal state.
    pub fn as_closable(&self) -> Closable {
        Closable(self.0.clone())
    }

    /// Returns an instance of [`SharedCloser`] that shares the same closing state.
    ///
    /// A shared closer returned by this method is `#[must_use]`, which means that it should be
    /// either managed, or discarded by [`SharedCloser::discard`]. Discarded shared closers
    /// associated with a parent [`Closer`] may trigger transition to a closed state.
    #[must_use]
    pub fn as_shared(&self) -> SharedCloser {
        SharedCloser {
            flag: self.0.clone(),
            owners: Arc::new(AtomicUsize::new(1)),
            associated: true,
        }
    }

    /// Closes the resource.
    ///
    /// From this moment both the main [`Closer`] and all associated instances of [`Closable`] will
    /// be closed.
    pub fn close(&self) {
        self.0.store(true, atomic::Ordering::Release);
    }

    /// Returns `true` if resource is closed.
    pub fn is_closed(&self) -> bool {
        self.0.load(atomic::Ordering::Acquire)
    }
}

impl Default for Closer {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Closer {
    fn drop(&mut self) {
        self.close()
    }
}

/// State of a resource or operation that handled by several independent subroutines.
///
/// Similar to [`Closer`], except it can be cloned. When all copies are gone out of scope, then
/// resource is transitioned to closed state.
///
/// The difference between [`SharedCloser`] and [`Arc<SharedCloser>`] is that the former can be
/// created from [`Closer`] by [`Closer::as_shared`]. In this scenario the resource will be
/// closed, when either original [`Closer`] is gone, or all copies of the dependent [`SharedCloser`]
/// have been destroyed.
#[derive(Debug)]
pub struct SharedCloser {
    flag: Arc<AtomicBool>,
    owners: Arc<AtomicUsize>,
    associated: bool,
}

impl SharedCloser {
    /// Creates a new instance of [`SharedCloser`].
    pub fn new() -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
            owners: Arc::new(AtomicUsize::new(1)),
            associated: false,
        }
    }

    /// Returns an instance of [`Closable`], a read-only accessor to the internal state.
    pub fn as_closable(&self) -> Closable {
        Closable(self.flag.clone())
    }

    /// Closes the resource.
    ///
    /// From this moment both the main [`Closer`] and all associated instances of [`Closable`] will
    /// be closed.
    pub fn close(&self) {
        self.flag.store(true, atomic::Ordering::Release);
    }

    /// Discards this shared closer without triggering a closing event.
    ///
    /// If shared closer was not produced by [`Closer::as_shared`], that means it is not associated
    /// and if this is a last copy of shared closer, it will trigger close event.
    ///
    /// This method is useful if caller receives a [`SharedCloser`] and don't want to trigger
    /// closing by dropping it.
    pub fn discard(mut self) {
        if !self.associated && self.owners.load(atomic::Ordering::Acquire) <= 1 {
            self.close();
        }
        let mut empty = Arc::new(AtomicBool::new(false));
        mem::swap(&mut empty, &mut self.flag);
    }

    /// Returns `true` if resource is closed.
    pub fn is_closed(&self) -> bool {
        self.flag.load(atomic::Ordering::Acquire)
    }
}

impl Default for SharedCloser {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for SharedCloser {
    fn clone(&self) -> Self {
        let owners = self.owners.clone();
        owners.fetch_add(1, atomic::Ordering::Release);

        Self {
            flag: self.flag.clone(),
            owners,
            associated: self.associated,
        }
    }
}

impl Drop for SharedCloser {
    fn drop(&mut self) {
        if self.owners.fetch_sub(1, atomic::Ordering::Release) <= 1 {
            self.flag.store(true, atomic::Ordering::Release);
        }
    }
}

/// Read-only access to a state of a resource.
///
/// [`Closable`] can be obtained by [`Closer::as_closable`] or [`SharedCloser::as_closable`].
/// This creates a read-only version of resource state.
#[derive(Clone, Debug)]
pub struct Closable(Arc<AtomicBool>);

impl Closable {
    /// Returns `true` if resource is closed.
    pub fn is_closed(&self) -> bool {
        self.0.load(atomic::Ordering::Acquire)
    }
}

#[cfg(test)]
mod test_closable {
    use super::*;

    #[test]
    fn closer_state_is_passing() {
        let closer = Closer::new();

        assert!(!closer.is_closed());

        let closable_1 = closer.as_closable();
        let closable_2 = closer.as_closable();

        assert!(!closable_1.is_closed());
        assert!(!closable_2.is_closed());

        closer.close();

        assert!(closer.is_closed());
        assert!(closable_1.is_closed());
        assert!(closable_2.is_closed());
    }

    #[test]
    fn closer_drop_means_closed() {
        let closer = Closer::new();

        let closable_1 = closer.as_closable();
        let closable_2 = closer.as_closable();

        drop(closer);

        assert!(closable_1.is_closed());
        assert!(closable_2.is_closed());
    }

    #[test]
    fn closer_arc_drop_mechanics() {
        let closer_1 = Arc::new(Closer::new());
        let closer_2 = closer_1.clone();

        let closable_1 = closer_1.as_closable();
        let closable_2 = closer_2.as_closable();

        drop(closer_1);

        assert!(!closable_2.is_closed());
        assert!(!closable_1.is_closed());
        assert!(!closable_2.is_closed());

        drop(closer_2);

        assert!(closable_1.is_closed());
        assert!(closable_2.is_closed());
    }

    #[test]
    fn dependent_shared_closers_may_trigger_close() {
        let closer = Closer::new();

        let shared_closer = closer.as_shared();
        drop(shared_closer);

        assert!(closer.is_closed());
    }

    #[test]
    fn shared_closers_behave_as_arc_closers() {
        let shared_closer = SharedCloser::new();

        let mut other_shared_closers = Vec::new();
        for _ in 0..100 {
            other_shared_closers.push(shared_closer.clone())
        }

        let closable_1 = shared_closer.as_closable();
        let closable_2 = closable_1.clone();

        for _ in 0..100 {
            other_shared_closers.pop();

            assert!(!shared_closer.is_closed());
            assert!(!closable_1.is_closed());
            assert!(!closable_2.is_closed());
        }

        drop(shared_closer);

        assert!(closable_1.is_closed());
        assert!(closable_2.is_closed());
    }

    #[test]
    fn dependent_shared_closers_can_be_discarded() {
        let closer = Closer::new();
        closer.as_shared().discard();
        assert!(!closer.is_closed());
    }

    #[test]
    fn drop_after_discard() {
        let closer = Closer::new();

        let shared_closer_1 = closer.as_shared();
        let shared_closer_2 = shared_closer_1.clone();

        shared_closer_1.discard();
        drop(shared_closer_2);

        assert!(closer.is_closed());
    }

    #[test]
    fn discard_after_drop() {
        let closer = Closer::new();

        let shared_closer_1 = closer.as_shared();
        let shared_closer_2 = shared_closer_1.clone();

        drop(shared_closer_1);
        shared_closer_2.discard();

        assert!(!closer.is_closed());
    }

    #[test]
    fn standalone_discard_closes() {
        let shared_closer = SharedCloser::new();
        let closable = shared_closer.as_closable();

        shared_closer.discard();
        assert!(closable.is_closed());
    }
}
