use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};

use crate::core::utils::TryUpdateFrom;
use crate::error::FrameError;
use crate::protocol::{CrcExtra, Frame, MavFrame, MaybeVersioned};

/// A protocol for custom frame processing.
pub trait ProcessFrame: Debug + Send + Sync {
    /// Processes provided frame according to the specified case.
    fn process(
        &mut self,
        frame: &mut MavFrame,
        case: ProcessFrameCase,
        crc_extra: Option<CrcExtra>,
    ) -> Result<(), FrameError>;
}

/// Defines a set of cases, when frame can be processed.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ProcessFrameCase {
    /// Ingoing frame before the default processing.
    IncomingBefore,
    /// Ingoing frame after the default processing.
    IncomingAfter,
    /// Outgoing frame before the default processing.
    OutgoingBefore,
    /// Outgoing frame after the default processing.
    OutgoingAfter,
}

/// Container for custom processors, that implement [`ProcessFrame`].
#[derive(Clone, Debug, Default)]
pub struct CustomFrameProcessors {
    inner: HashMap<&'static str, Arc<Mutex<dyn ProcessFrame>>>,
    sorted_keys: Vec<&'static str>,
    sorted_keys_rev: Vec<&'static str>,
}

impl CustomFrameProcessors {
    /// Returns `true` if there are no available frame processors.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Adds a new processor with specified `name`.
    ///
    /// Names should be unique within a collection. If the new processor with the same name is
    /// provided, then the older processor will be overwritten. Check [`process`] method for more
    /// details on how processors will be applied.
    ///
    /// [`process`]: Self::process
    pub fn add(&mut self, name: &'static str, processor: impl ProcessFrame + 'static) {
        self.inner.insert(name, Arc::new(Mutex::new(processor)));
        self.resort_keys();
    }

    /// Processes a [`Frame`] according to the provided [`ProcessFrameCase`] and optional
    /// `crc_extra`.
    ///
    /// Processors will be applied in alphabetical order according to their names for
    /// [`IncomingBefore`] and [`OutgoingBefore`] and in the reverse alphabetical order for
    /// [`IncomingAfter`] and [`OutgoingAfter`]. This means that processors can "undo" what they
    /// have done to frames in the correct order.
    ///
    /// [`IncomingBefore`]: ProcessFrameCase::IncomingBefore
    /// [`OutgoingBefore`]: ProcessFrameCase::OutgoingBefore
    /// [`IncomingAfter`]: ProcessFrameCase::IncomingAfter
    /// [`OutgoingAfter`]: ProcessFrameCase::OutgoingAfter
    pub fn process<V: MaybeVersioned>(
        &self,
        frame: &mut Frame<V>,
        case: ProcessFrameCase,
        crc_extra: Option<CrcExtra>,
    ) -> Result<(), FrameError> {
        if self.inner.is_empty() {
            return Ok(());
        }

        let mut mav_frame = frame.clone().into_mav_frame();

        let keys = match case {
            ProcessFrameCase::IncomingBefore | ProcessFrameCase::OutgoingBefore => {
                self.sorted_keys.iter()
            }
            ProcessFrameCase::IncomingAfter | ProcessFrameCase::OutgoingAfter => {
                self.sorted_keys_rev.iter()
            }
        };

        for name in keys {
            let processor = self.inner.get(name).unwrap();

            if let Ok(mut processor) = processor.lock() {
                processor.process(&mut mav_frame, case, crc_extra)?;

                if let Err(err) = frame.try_update_from(&mav_frame) {
                    log::error!("[frame processor] invalid output from custom processor '{name}' for {case:?}: {err:?}");
                    return Err(FrameError::from(err));
                }
            }
        }

        Ok(())
    }

    pub(super) fn extend(&mut self, other: &Self) {
        for (name, processor) in &other.inner {
            self.inner.insert(name, processor.clone());
        }
        self.resort_keys();
    }

    fn resort_keys(&mut self) {
        self.sorted_keys = self.inner.keys().copied().collect();
        self.sorted_keys.sort();
        self.sorted_keys_rev = self.sorted_keys.iter().rev().copied().collect();
    }
}
