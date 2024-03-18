use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};

use crate::core::error::FrameError;
use crate::core::utils::TryUpdateFrom;
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
    /// provided, then the older processor will be overwritten.
    pub fn add(&mut self, name: &'static str, processor: impl ProcessFrame + 'static) {
        self.inner.insert(name, Arc::new(Mutex::new(processor)));
    }

    /// Processes a [`Frame`] according to the provided [`ProcessFrameCase`] and optional
    /// `crc_extra`.
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

        for (name, processor) in self.inner.iter() {
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
}
