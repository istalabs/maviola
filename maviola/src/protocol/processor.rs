use std::fmt::{Debug, Formatter};

use crate::error::FrameError;
#[cfg(feature = "unsafe")]
use crate::protocol::ProcessFrameCase;
use crate::protocol::{
    CompatProcessor, CustomFrameProcessors, DialectSpec, Frame, FrameSigner, KnownDialects,
    MaybeVersioned,
};

#[cfg(doc)]
use crate::core::node::Node;
#[cfg(doc)]
use crate::protocol::Device;

/// Process MAVLink frames according to protocol, dialect, and additional rules.
///
/// Frame processor is responsible for managing frame signature, incompatibility/compatibility
/// flags, validating frames against known dialects and CRC, and other checks defined by a set of
/// rules.
///
/// ⓘ Frame processors are used by [`Node`] and [`Device`] internally to produce and process frames.
/// However, they never expose them. The reason is that frame processing is not generally idempotent.
/// Which means you may render a frame useless by applying processor twice. Still we've found this
/// abstraction handy and provide it for those who may want to extend Maviola functionality.
#[derive(Default)]
pub struct FrameProcessor {
    compat: Option<CompatProcessor>,
    signer: Option<FrameSigner>,
    dialects: KnownDialects,
    #[cfg(feature = "unsafe")]
    processors: CustomFrameProcessors,
}

/// Builder for [`FrameProcessor`].
#[derive(Clone, Default)]
pub struct FrameProcessorBuilder {
    compat: Option<CompatProcessor>,
    signer: Option<FrameSigner>,
    dialects: KnownDialects,
    #[cfg(feature = "unsafe")]
    processors: CustomFrameProcessors,
}

impl FrameProcessor {
    /// Creates an empty builder for the frame processor.
    pub(crate) fn builder() -> FrameProcessorBuilder {
        FrameProcessorBuilder::default()
    }

    /// Returns an optional reference to a [`FrameSigner`].
    pub fn signer(&self) -> Option<&FrameSigner> {
        self.signer.as_ref()
    }

    /// Returns an optional reference to a [`CompatProcessor`].
    pub fn compat(&self) -> Option<&CompatProcessor> {
        self.compat.as_ref()
    }

    /// Main dialect specification.
    #[inline(always)]
    pub fn main_dialect(&self) -> &DialectSpec {
        self.dialects.main()
    }

    /// Supported dialect specifications.
    #[inline(always)]
    pub fn known_dialects(&self) -> impl Iterator<Item = &DialectSpec> {
        self.dialects.known()
    }

    /// Prepares a new outgoing frame.
    pub fn process_new<V: MaybeVersioned>(&self, frame: &mut Frame<V>) {
        if let Some(signer) = &self.signer {
            signer.process_new(frame);
        }
    }

    /// Takes incoming frame and processes it according to defined signing and compatibility
    /// settings.
    pub fn process_incoming<V: MaybeVersioned>(
        &self,
        frame: &mut Frame<V>,
    ) -> Result<(), FrameError> {
        #[cfg(feature = "unsafe")]
        self.apply_custom_processors(frame, ProcessFrameCase::IncomingBefore)?;

        if let Some(compat) = &self.compat {
            if let Err(err) = compat.process_incoming(frame, self.dialects.as_slice()) {
                self.check_compat_err(err)?;
            }
        }

        if let Some(signer) = &self.signer {
            signer.process_incoming(frame)?;
        }

        #[cfg(feature = "unsafe")]
        self.apply_custom_processors(frame, ProcessFrameCase::IncomingAfter)?;
        Ok(())
    }

    /// Takes outgoing frame and processes it according to defined signing and compatibility
    /// settings.
    pub fn process_outgoing<V: MaybeVersioned>(
        &self,
        frame: &mut Frame<V>,
    ) -> Result<(), FrameError> {
        #[cfg(feature = "unsafe")]
        self.apply_custom_processors(frame, ProcessFrameCase::OutgoingBefore)?;

        if let Some(compat) = &self.compat {
            if let Err(err) = compat.process_outgoing(frame, self.dialects.as_slice()) {
                self.check_compat_err(err)?;
            }
        }

        if let Some(signer) = &self.signer {
            signer.process_outgoing(frame)?;
        }

        #[cfg(feature = "unsafe")]
        self.apply_custom_processors(frame, ProcessFrameCase::OutgoingAfter)?;
        Ok(())
    }

    fn check_compat_err(&self, err: FrameError) -> Result<(), FrameError> {
        match err {
            FrameError::NotInDialect(_) if self.dialects.allow_unknown() => Ok(()),
            err => Err(err),
        }
    }

    #[cfg(feature = "unsafe")]
    /// <sup>⛔ | 💢</sup>
    /// Applies custom processors. Works only when `unsafe` feature is enabled.
    fn apply_custom_processors<V: MaybeVersioned>(
        &self,
        frame: &mut Frame<V>,
        case: ProcessFrameCase,
    ) -> Result<(), FrameError> {
        if self.processors.is_empty() {
            return Ok(());
        }

        let crc_extra = self
            .dialects
            .message_info_by_id(frame.message_id())
            .map(|info| info.crc_extra());

        if crc_extra.is_none() && !self.dialects.allow_unknown() {
            return Err(FrameError::NotInDialect(frame.message_id()));
        }

        self.processors.process(frame, case, crc_extra)
    }

    /// <sup>⛔</sup>
    /// Extends the current frame processor with the settings from the provided one.
    pub(crate) fn extend_with(&mut self, other: &FrameProcessor) {
        #[cfg(feature = "unsafe")]
        self.processors.extend(&other.processors);

        self.dialects.append_known_dialects(&other.dialects);

        if self.signer.is_none() {
            if let Some(signer) = other.signer() {
                self.signer = Some(signer.clone());
            }
        }

        if self.compat.is_none() {
            if let Some(compat) = other.compat() {
                self.compat = Some(compat.clone());
            }
        }
    }
}

impl Debug for FrameProcessor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrameProcessor").finish_non_exhaustive()
    }
}

impl FrameProcessorBuilder {
    /// Builds a [`FrameProcessor`] from internal configuration.
    #[cfg(feature = "unsafe")]
    pub fn build(self) -> FrameProcessor {
        FrameProcessor {
            compat: self.compat,
            signer: self.signer,
            dialects: self.dialects,
            processors: self.processors,
        }
    }

    /// Builds a [`FrameProcessor`] from internal configuration.
    #[cfg(not(feature = "unsafe"))]
    pub fn build(self) -> FrameProcessor {
        FrameProcessor {
            compat: self.compat,
            signer: self.signer,
            dialects: self.dialects,
        }
    }

    /// Adds a [`FrameSigner`] to a processor.
    ///
    /// When used with [`FrameProcessor::compat`], will set
    /// [`CompatProcessor::ignore_signature`] to `true` trusting the signer to handle message
    /// signing incompatibility flag.
    pub fn signer(mut self, signer: FrameSigner) -> Self {
        self.signer = Some(signer);
        if let Some(compat) = self.compat {
            self.compat = Some(compat.update().ignore_signature(true).build());
        }
        self
    }

    /// Adds a [`CompatProcessor`] to a processor.
    ///
    /// When used with [`FrameProcessor::signer`], then [`CompatProcessor::ignore_signature`]
    /// will be set to `true` trusting the signer to handle message signing incompatibility flag.
    pub fn compat(mut self, compat: CompatProcessor) -> Self {
        let compat = match self.signer {
            None => compat,
            Some(_) => compat.update().ignore_signature(true).build(),
        };
        self.compat = Some(compat);
        self
    }

    /// Adds [`KnownDialects`] to a processor.
    pub fn dialects(mut self, dialects: KnownDialects) -> Self {
        self.dialects = dialects;
        self
    }

    /// <sup>💢</sup>
    /// Sets custom processors, that implement [`ProcessFrame`].
    #[cfg(feature = "unsafe")]
    pub fn processors(mut self, processors: CustomFrameProcessors) -> Self {
        self.processors = processors;
        self
    }

    /// <sup>⛔</sup>
    /// Sets custom processors (does nothing if `unsafe` feature is not enabled).
    #[cfg(not(feature = "unsafe"))]
    pub(crate) fn processors(self, _: CustomFrameProcessors) -> Self {
        self
    }
}

#[cfg(test)]
mod processor_tests {
    use super::*;
    use crate::protocol::{IncompatFlags, SecretKey};

    #[test]
    fn extend_processor_new_signer() {
        let other = FrameProcessor::builder()
            .signer(FrameSigner::new(1, "abc"))
            .build();
        let mut this = FrameProcessor::builder().build();

        this.extend_with(&other);

        assert!(this.signer().is_some());
    }

    #[test]
    fn extend_processor_keep_signer() {
        let other = FrameProcessor::builder()
            .signer(FrameSigner::new(1, "abc"))
            .build();
        let mut this = FrameProcessor::builder()
            .signer(FrameSigner::new(1, "abcd"))
            .build();

        this.extend_with(&other);

        assert!(this.signer().is_some());
        assert_eq!(
            this.signer().unwrap().key().value(),
            SecretKey::from("abcd").value()
        );
    }

    #[test]
    fn extend_processor_new_compat() {
        let other = FrameProcessor::builder()
            .compat(CompatProcessor::builder().build())
            .build();
        let mut this = FrameProcessor::builder().build();

        this.extend_with(&other);

        assert!(this.compat().is_some());
    }

    #[test]
    fn extend_processor_keep_compat() {
        let other = FrameProcessor::builder()
            .compat(
                CompatProcessor::builder()
                    .incompat_flags(IncompatFlags::MAVLINK_IFLAG_SIGNED)
                    .build(),
            )
            .build();
        let mut this = FrameProcessor::builder()
            .compat(
                CompatProcessor::builder()
                    .incompat_flags(IncompatFlags::BIT_2)
                    .build(),
            )
            .build();

        this.extend_with(&other);

        assert!(this.compat().is_some());
        assert!(this.compat().unwrap().incompat_flags().is_some());
        assert_eq!(
            this.compat().unwrap().incompat_flags().unwrap(),
            IncompatFlags::BIT_2
        );
    }
}
