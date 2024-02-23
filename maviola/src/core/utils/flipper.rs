use std::marker::PhantomData;
use std::sync::atomic::AtomicBool;
use std::sync::{atomic, Arc};

use crate::core::utils::closable::WillClose;
use crate::core::utils::{Closable, Closer, Sealed, SharedCloser};

/// <sup>🔒</sup>
/// A trait that represents a shared atomic guarded boolean value that can be finalized.
///
/// ⚠ This trait is sealed ⚠
pub trait Flipper: Sealed {}

/// A simple flag that can be either in "on" or "off" state.
///
/// Combined with a [`Guarded`], it will be flipped on drop of the guard.
#[derive(Debug, Default)]
pub struct Flag;

/// A switch that can be either in "on" or "off" state and has a predefined final state.
///
/// Combined with a [`Guarded`], it will be set to the final state, when guard is dropped.
#[derive(Debug, Default)]
pub struct Switch;

/// Guarded flipper.
pub struct Guarded<C: WillClose, F: Flipper> {
    flag: Arc<AtomicBool>,
    guard: C,
    state: Closable,
    _kind: PhantomData<F>,
}

impl Guarded<Closer, Switch> {
    /// Default constructor.
    ///
    /// Creates a guarded switch which is initialized with `false` and will be set to `false` once
    /// guard is dropped.
    ///
    /// # Usage
    ///
    /// ```rust
    /// use maviola::core::utils::Guarded;
    ///
    /// let switch = Guarded::new();
    /// assert!(!switch.is());
    /// ```
    pub fn new() -> Self {
        let guard = Closer::new();
        let state = guard.to_closable();

        Self {
            flag: Arc::new(AtomicBool::new(false)),
            guard,
            state,
            _kind: PhantomData,
        }
    }

    /// Creates default shared switch.
    ///
    /// Default switch is initialized with `false` and will be set to `false` once
    /// guard is dropped.
    ///
    /// # Usage
    ///
    /// ```rust
    /// use maviola::core::utils::Guarded;
    ///
    /// let switch_1 = Guarded::shared();
    /// let mut switch_2 = switch_1.clone();
    ///
    /// switch_2.set(true);
    /// assert!(switch_1.is());
    /// ```
    pub fn shared() -> Guarded<SharedCloser, Switch> {
        Self::new().into_shared()
    }

    /// Sets the value of the switch to `true` in-place, does not have effect if already closed.
    ///
    /// This method takes [`Guarded<Closer, Switch>`] by value and returns an updated version.
    ///
    /// # Usage
    ///
    /// ```rust
    /// use maviola::core::utils::Guarded;
    ///
    /// let switch = Guarded::new().up();
    /// assert!(switch.is());
    /// ```
    pub fn up(self) -> Self {
        if !self.state.is_closed() {
            self.flag.store(true, atomic::Ordering::Release);
        }

        Self {
            flag: self.flag,
            guard: self.guard,
            state: self.state,
            _kind: PhantomData,
        }
    }

    /// Create a shared flag from this switch.
    #[must_use]
    pub fn to_flag(&self) -> Guarded<SharedCloser, Flag> {
        self.to_shared().into_flag()
    }
}

impl<F: Flipper> Guarded<Closer, F> {
    /// Creates a new associated shared guarded flipper.
    ///
    /// the result of this method is marked as `#[must_use]` since if the obtained shared flipper
    /// will be dropped, then original flipper will also receive a closing event.
    #[must_use]
    pub fn to_shared(&self) -> Guarded<SharedCloser, F> {
        Guarded {
            flag: self.flag.clone(),
            guard: self.guard.to_shared(),
            state: self.state.clone(),
            _kind: PhantomData,
        }
    }

    /// Transforms itself into shared guarded flipper.
    pub fn into_shared(self) -> Guarded<SharedCloser, F> {
        Guarded {
            flag: self.flag.clone(),
            guard: self.guard.into_shared(),
            state: self.state.clone(),
            _kind: PhantomData,
        }
    }

    /// Closes the guarded flipper.
    pub fn close(&mut self) {
        self.guard.close()
    }
}

impl<F: Flipper> Guarded<SharedCloser, F> {
    /// Converts this shared closer into a shared flag.
    pub fn into_flag(self) -> Guarded<SharedCloser, Flag> {
        Guarded {
            flag: self.flag.clone(),
            guard: self.guard,
            state: self.state,
            _kind: PhantomData,
        }
    }

    /// Discards this shared guard without triggering a closing event.
    pub fn discard(self) {
        self.guard.discard()
    }

    /// Closes the guarded flipper.
    pub fn close(&mut self) {
        self.guard.close()
    }
}

impl<C: WillClose> Guarded<C, Switch> {
    /// Sets the value of the switch, does not have effect if already closed.
    ///
    /// This method takes [`Guarded<SharedCloser, Switch>`] by mutable reference.
    pub fn set(&mut self, value: bool) {
        if !self.state.is_closed() {
            self.flag.store(value, atomic::Ordering::Release);
        }
    }
}

impl<C: WillClose, F: Flipper> Guarded<C, F> {
    /// Creates a watcher for a flipper.
    ///
    /// A watcher is an instance of [`Guarded<Closable, _>`] which can't initiate closing.
    pub fn to_watcher(&self) -> Guarded<Closable, Flag> {
        Guarded {
            flag: self.flag.clone(),
            guard: self.state.clone(),
            state: self.state.clone(),
            _kind: PhantomData,
        }
    }

    /// Returns the value of a switch.
    ///
    /// Always returns `false` if guard is closed.
    pub fn is(&self) -> bool {
        if self.state.is_closed() {
            false
        } else {
            self.flag.load(atomic::Ordering::Acquire)
        }
    }

    /// Returns `true` if guard is dropped or closed.
    pub fn is_closed(&self) -> bool {
        self.guard.is_closed()
    }
}

impl Default for Guarded<Closer, Switch> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F: Flipper> Clone for Guarded<SharedCloser, F> {
    fn clone(&self) -> Self {
        Self {
            flag: self.flag.clone(),
            guard: self.guard.clone(),
            state: self.state.clone(),
            _kind: PhantomData,
        }
    }
}

impl<F: Flipper> Clone for Guarded<Closable, F> {
    fn clone(&self) -> Self {
        Self {
            flag: self.flag.clone(),
            guard: self.guard.clone(),
            state: self.state.clone(),
            _kind: PhantomData,
        }
    }
}

impl<C: WillClose, F: Flipper> Sealed for Guarded<C, F> {}
impl<C: WillClose, F: Flipper> WillClose for Guarded<C, F> {
    fn is_closed(&self) -> bool {
        self.is_closed()
    }

    fn to_closable(&self) -> Closable {
        self.state.to_closable()
    }
}

impl Sealed for Flag {}
impl Flipper for Flag {}

impl Sealed for Switch {}
impl Flipper for Switch {}

impl From<Closer> for Guarded<Closer, Switch> {
    fn from(value: Closer) -> Self {
        let state = value.to_closable();
        Self {
            flag: Arc::new(AtomicBool::new(false)),
            guard: value,
            state,
            _kind: PhantomData,
        }
    }
}

impl From<&Closer> for Guarded<SharedCloser, Switch> {
    fn from(value: &Closer) -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
            guard: value.to_shared(),
            state: value.to_closable(),
            _kind: PhantomData,
        }
    }
}

impl From<SharedCloser> for Guarded<SharedCloser, Switch> {
    fn from(value: SharedCloser) -> Self {
        let state = value.to_closable();
        Self {
            flag: Arc::new(AtomicBool::new(false)),
            guard: value,
            state,
            _kind: PhantomData,
        }
    }
}

impl From<&SharedCloser> for Guarded<SharedCloser, Switch> {
    fn from(value: &SharedCloser) -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
            guard: value.clone(),
            state: value.to_closable(),
            _kind: PhantomData,
        }
    }
}

#[cfg(test)]
mod test_flipper {
    use super::*;

    #[test]
    fn basic_switch_workflow() {
        let switch = Guarded::new();
        assert!(!switch.is());

        let switch = Guarded::new().up();
        assert!(switch.is());

        let mut switch = Guarded::new();
        switch.set(true);
        assert!(switch.is());
        switch.set(false);
        assert!(!switch.is());

        let switch = Guarded::new().up();
        let watcher = switch.to_watcher();

        assert!(watcher.is());
        drop(switch);
        assert!(!watcher.is());
    }

    #[test]
    fn shared_switch_workflow() {
        let mut switch = Guarded::new();
        let mut shared_switch = switch.to_shared();

        assert!(!shared_switch.is());

        switch.set(true);
        assert!(switch.is());
        assert!(shared_switch.is());

        shared_switch.set(false);
        assert!(!switch.is());
        assert!(!shared_switch.is());

        let switch = Guarded::new().up();
        let shared_switch = switch.to_shared();
        drop(switch);
        assert!(!shared_switch.is());

        let switch = Guarded::new().up();
        let shared_switch = switch.to_shared();
        drop(shared_switch);
        assert!(!switch.is());
    }

    #[test]
    fn state_between_shared_switches_is_shared() {
        let shared_switch_1 = Guarded::shared();
        let mut shared_switch_2 = shared_switch_1.clone();

        assert!(!shared_switch_1.is_closed());
        assert!(!shared_switch_2.is_closed());

        shared_switch_2.set(true);
        assert!(shared_switch_2.is());
        assert!(shared_switch_1.is());
    }

    #[test]
    fn to_shared_switch_discard() {
        let switch = Guarded::new().up();
        switch.to_shared().discard();
        assert!(switch.is());

        let switch = Guarded::new().up();
        _ = switch.to_shared();
        assert!(!switch.is());
    }

    #[test]
    fn test_shared_flags() {
        let mut switch = Guarded::new();
        let flag = switch.to_flag();

        switch.set(true);
        assert!(flag.is());

        drop(flag);
        assert!(switch.is_closed());

        let switch = Guarded::new();
        let flag = switch.to_flag();

        drop(switch);
        assert!(flag.is_closed());
    }

    #[test]
    fn test_watcher_flags() {
        let mut switch = Guarded::new();
        let flag = switch.to_watcher();

        switch.set(true);
        assert!(flag.is());

        drop(switch);
        assert!(flag.is_closed());
        assert!(!flag.is());
    }
}
