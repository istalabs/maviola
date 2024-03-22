//! # Abstractions for closable tasks and resources
//!
//! This module provides abstractions for resources which state has to be tracked in distributed
//! computations.
//!
//! There are three level of resource ownership:
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
//! All eventually "closable" entities implement trait [`WillClose`], as well as [`Closed`], a
//! constant type which is always closed.
//!
//! It is possible to close [`Closer`] and [`SharedCloser`] prematurely. Still, this requires
//! mutable access to the instance. That means that owner of a closer will have access over what
//! borrowers can and can't do with the state.
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

use crate::core::utils::Sealed;

/// <sup>🔒</sup>
/// A trait for anything that may be closed.
///
/// 🔒 This trait is sealed 🔒
///
/// This trait is implemented by [`Closer`], [`SharedCloser`], [`Closable`], and [`Closed`].
pub trait WillClose: Sealed {
    /// Returns `true` if resource is closed.
    ///
    /// The blanket implementation always returns `true`.
    fn is_closed(&self) -> bool {
        true
    }

    /// Returns [`Closable`] proxy to itself.
    fn to_closable(&self) -> Closable;
}

/// State of a resource or operation that governed by the main task.
///
/// [`Closer`] is owned by a resource that can be closed. When [`Closer::close`] is called or
/// object goes out of scope, it becomes closed. Owner of [`Closer`] can share a read-only
/// view of the "closed" state to multiple listeners. Read-only access is provided by [`Closable`]
/// which can be obtained by calling [`Closer::to_closable`].
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
/// let closable_1 = closer.to_closable();
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
/// let closable_1 = closer_1.to_closable();
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
#[must_use]
pub struct Closer(Arc<AtomicBool>);

impl Closer {
    /// Creates an instance of closable.
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    /// Returns an instance of [`Closable`], a read-only accessor to the internal state.
    pub fn to_closable(&self) -> Closable {
        Closable(self.0.clone())
    }

    /// Returns an instance of [`SharedCloser`] that shares the same closing state.
    ///
    /// The result of this method marked as `#[must_use]` since dropping an obtained shared
    /// closed will also trigger undesirable closing. A caller should either manage, or explicitly
    /// discard the obtained shared closer by [`SharedCloser::discard`]. Non-discarded shared
    /// closers associated with a parent [`Closer`] will trigger a shared transition to a closed
    /// state when all of them are dropped.
    ///
    /// If you want to obtain a [`SharedCloser`] from a [`Closer`] by dropping the latter without
    /// triggering a close event, use [`Closer::into_shared`] which takes closer by value.
    ///
    /// # Examples
    ///
    /// This is a correct usage:
    ///
    /// ```rust
    /// use maviola::core::utils::Closer;
    ///
    /// let mut closer = Closer::new();
    /// let shared = closer.to_shared();
    ///
    /// assert!(!closer.is_closed());
    /// assert!(!shared.is_closed());
    /// ```
    ///
    /// But the following will drop the initial closer and will trigger a closing event immediately:
    ///
    /// ```rust
    /// use maviola::core::utils::Closer;
    ///
    /// let shared = Closer::new().to_shared();
    ///
    /// assert!(shared.is_closed());
    /// ```
    ///
    /// This will also trigger an undesired close event:
    ///
    /// ```rust
    /// use maviola::core::utils::Closer;
    ///
    /// let closer = Closer::new();
    /// let _ = closer.to_shared();
    /// ```
    pub fn to_shared(&self) -> SharedCloser {
        SharedCloser {
            flag: self.0.clone(),
            owners: Arc::new(AtomicUsize::new(1)),
            associated: true,
        }
    }

    /// Transforms into a shared closer.
    ///
    /// Takes an instance of [`Closer`] by value and transforms it into a [`SharedCloser`].
    pub fn into_shared(mut self) -> SharedCloser {
        let mut flag = Arc::new(AtomicBool::new(false));
        mem::swap(&mut self.0, &mut flag);

        SharedCloser {
            flag,
            owners: Arc::new(AtomicUsize::new(1)),
            associated: false,
        }
    }

    /// Closes the resource.
    ///
    /// From this moment both the main [`Closer`] and all associated instances of [`Closable`] will
    /// be closed.
    pub fn close(&mut self) {
        self.0.store(true, atomic::Ordering::Release);
    }

    /// Returns `true` if resource is closed.
    pub fn is_closed(&self) -> bool {
        self.0.load(atomic::Ordering::Acquire)
    }
}

impl Sealed for Closer {}
impl WillClose for Closer {
    fn is_closed(&self) -> bool {
        self.is_closed()
    }

    fn to_closable(&self) -> Closable {
        self.to_closable()
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
/// created from [`Closer`] by [`Closer::to_shared`]. In this scenario the resource will be
/// closed, when either original [`Closer`] is gone, or all copies of the dependent [`SharedCloser`]
/// have been destroyed.
#[derive(Debug)]
#[must_use]
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
    pub fn to_closable(&self) -> Closable {
        Closable(self.flag.clone())
    }

    /// Closes the resource.
    ///
    /// From this moment both the main [`Closer`] and all associated instances of [`Closable`] will
    /// be closed.
    pub fn close(&mut self) {
        self.flag.store(true, atomic::Ordering::Release);
    }

    /// Discards this shared closer without triggering a closing event.
    ///
    /// If shared closer was not produced by [`Closer::to_shared`], that means it is not associated
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

impl Sealed for SharedCloser {}
impl WillClose for SharedCloser {
    fn is_closed(&self) -> bool {
        self.is_closed()
    }

    fn to_closable(&self) -> Closable {
        self.to_closable()
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
/// [`Closable`] can be obtained by [`Closer::to_closable`] or [`SharedCloser::to_closable`].
/// This creates a read-only version of resource state.
#[derive(Clone, Debug)]
#[must_use]
pub struct Closable(Arc<AtomicBool>);

impl Closable {
    /// Returns `true` if resource is closed.
    pub fn is_closed(&self) -> bool {
        self.0.load(atomic::Ordering::Acquire)
    }
}

impl Sealed for Closable {}
impl WillClose for Closable {
    #[inline(always)]
    fn is_closed(&self) -> bool {
        self.is_closed()
    }

    fn to_closable(&self) -> Closable {
        self.clone()
    }
}

/// An implementor of [`WillClose`], that always closed.
#[derive(Copy, Clone, Debug, Default)]
pub struct Closed;

impl Closed {
    /// Always returns `true`.
    pub const fn is_closed(&self) -> bool {
        true
    }

    /// Returns already closed [`Closable`].
    pub fn to_closable(&self) -> Closable {
        Closable(Arc::new(AtomicBool::new(false)))
    }
}

impl Sealed for Closed {}

impl WillClose for Closed {
    #[inline(always)]
    fn is_closed(&self) -> bool {
        self.is_closed()
    }

    fn to_closable(&self) -> Closable {
        self.to_closable()
    }
}

#[cfg(test)]
mod test_closable {
    use super::*;

    #[test]
    fn closer_state_is_passing() {
        let mut closer = Closer::new();

        assert!(!closer.is_closed());

        let closable_1 = closer.to_closable();
        let closable_2 = closer.to_closable();

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

        let closable_1 = closer.to_closable();
        let closable_2 = closer.to_closable();

        drop(closer);

        assert!(closable_1.is_closed());
        assert!(closable_2.is_closed());
    }

    #[test]
    fn closer_arc_drop_mechanics() {
        let closer_1 = Arc::new(Closer::new());
        let closer_2 = closer_1.clone();

        let closable_1 = closer_1.to_closable();
        let closable_2 = closer_2.to_closable();

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

        let shared_closer = closer.to_shared();
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

        let closable_1 = shared_closer.to_closable();
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
        closer.to_shared().discard();
        assert!(!closer.is_closed());
    }

    #[test]
    fn drop_after_discard() {
        let closer = Closer::new();

        let shared_closer_1 = closer.to_shared();
        let shared_closer_2 = shared_closer_1.clone();

        shared_closer_1.discard();
        drop(shared_closer_2);

        assert!(closer.is_closed());
    }

    #[test]
    fn discard_after_drop() {
        let closer = Closer::new();

        let shared_closer_1 = closer.to_shared();
        let shared_closer_2 = shared_closer_1.clone();

        drop(shared_closer_1);
        shared_closer_2.discard();

        assert!(!closer.is_closed());
    }

    #[test]
    fn standalone_discard_closes() {
        let shared_closer = SharedCloser::new();
        let closable = shared_closer.to_closable();

        shared_closer.discard();
        assert!(closable.is_closed());
    }
}
