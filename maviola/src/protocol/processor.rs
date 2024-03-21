use std::fmt::{Debug, Formatter};

use crate::error::FrameError;
#[cfg(feature = "unsafe")]
use crate::protocol::ProcessFrameCase;
use crate::protocol::{
    CompatProcessor, CustomFrameProcessors, DialectSpec, Frame, FrameSigner, KnownDialects,
    MaybeVersioned,
};

/// Process MAVLink frames according to protocol, dialect, and additional rules.
///
/// Frame processor is responsible for managing frame signature, incompatibility/compatibility
/// flags, validating frames against known dialects and CRC, and other checks defined by a set of
/// rules.
#[derive(Clone, Default)]
pub struct FrameProcessor {
    compat: Option<CompatProcessor>,
    signer: Option<FrameSigner>,
    dialects: KnownDialects,
    processors: CustomFrameProcessors,
}

/// Builder for [`FrameProcessor`].
#[derive(Clone, Default)]
pub struct FrameProcessorBuilder {
    inner: FrameProcessor,
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
}

impl Debug for FrameProcessor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrameProcessor").finish_non_exhaustive()
    }
}

impl FrameProcessorBuilder {
    /// Builds a [`FrameProcessor`] from internal configuration.
    pub fn build(self) -> FrameProcessor {
        self.inner
    }

    /// Adds a [`FrameSigner`] to a processor.
    ///
    /// When used with [`FrameProcessor::compat`], will set
    /// [`CompatProcessor::ignore_signature`] to `true` trusting the signer to handle message
    /// signing incompatibility flag.
    pub fn signer(mut self, signer: FrameSigner) -> Self {
        self.inner.signer = Some(signer);
        if let Some(compat) = self.inner.compat {
            self.inner.compat = Some(compat.update().ignore_signature(true).build());
        }
        self
    }

    /// Adds a [`CompatProcessor`] to a processor.
    ///
    /// When used with [`FrameProcessor::signer`], then [`CompatProcessor::ignore_signature`]
    /// will be set to `true` trusting the signer to handle message signing incompatibility flag.
    pub fn compat(mut self, compat: CompatProcessor) -> Self {
        let compat = match self.inner.signer {
            None => compat,
            Some(_) => compat.update().ignore_signature(true).build(),
        };
        self.inner.compat = Some(compat);
        self
    }

    /// Adds [`KnownDialects`] to a processor.
    pub fn dialects(mut self, dialects: KnownDialects) -> Self {
        self.inner.dialects = dialects;
        self
    }

    /// Sets custom processors, that implement [`ProcessFrame`].
    #[cfg(feature = "unsafe")]
    pub fn processors(mut self, processors: CustomFrameProcessors) -> Self {
        self.inner.processors = processors;
        self
    }

    #[cfg(not(feature = "unsafe"))]
    pub(crate) fn processors(self, _: CustomFrameProcessors) -> Self {
        self
    }
}
