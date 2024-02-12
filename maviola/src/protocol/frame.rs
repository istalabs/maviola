//! MAVLink frame.

use std::fmt::{Debug, Formatter};

use mavio::protocol::{
    Checksum, CompatFlags, DialectImpl, DialectMessage, IncompatFlags, MavLinkVersion,
    MavTimestamp, MaybeVersioned, MessageId, Payload, Signature, SignatureLinkId, Versionless, V1,
    V2,
};

use crate::prelude::*;

use crate::protocol::variants::{Dialectless, HasDialect, MaybeDialect};

/// Basic MAVLink frame.
///
/// Currently, this is simply an alias for [`mavio::Frame`].
pub type CoreFrame<V> = mavio::Frame<V>;

/// MAVLink frame potentially restricted to a specific dialect and MAVLink protocol version.
///
/// This is a wrapper around [`CoreFrame`] that allows to decode MAVLink message in the context
/// of a specific MAVLink dialect.
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Frame<D: MaybeDialect, V: MaybeVersioned> {
    frame: CoreFrame<V>,
    dialect: D,
}

impl<V: MaybeVersioned> Debug for Frame<Dialectless, V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Frame").field("frame", &self.frame).finish()
    }
}

impl<M: DialectMessage + 'static, V: MaybeVersioned> Debug for Frame<HasDialect<M>, V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let dialect = f
            .debug_struct("Dialect")
            .field("name", &self.dialect.0.name())
            .finish_non_exhaustive();

        f.debug_struct("Frame")
            .field("frame", &self.frame)
            .field("dialect", &dialect)
            .finish()
    }
}

impl<D: MaybeDialect, V: MaybeVersioned> From<Frame<D, V>> for CoreFrame<V> {
    /// Converts [`Frame`] into [`CoreFrame`].
    fn from(value: Frame<D, V>) -> Self {
        value.frame
    }
}

impl<V: MaybeVersioned> From<CoreFrame<V>> for Frame<Dialectless, V> {
    /// Converts [`CoreFrame`] into [`Frame`].
    fn from(value: CoreFrame<V>) -> Self {
        Self::new(value)
    }
}

impl<V: MaybeVersioned> Frame<Dialectless, V> {
    /// Create an instance of [`Frame`] without neither a specified dialect, nor MAVLink protocol
    /// version.
    ///
    /// Use [`Frame::builder`] to construct frames with additional restrictions.
    pub fn new(frame: CoreFrame<V>) -> Frame<Dialectless, V> {
        Self {
            frame,
            dialect: Dialectless,
        }
    }
}

impl Frame<Dialectless, Versionless> {
    /// Instantiate an empty instance of a [`FrameBuilder`].
    pub fn builder() -> FrameBuilder<Dialectless, Versionless> {
        FrameBuilder::new()
    }
}

impl<M: DialectMessage + 'static, V: MaybeVersioned> Frame<HasDialect<M>, V> {
    /// Decodes MAVLink frame within a specified dialect.
    pub fn decode(&self) -> Result<M> {
        self.dialect
            .0
            .decode(self.frame.payload())
            .map_err(Error::from)
    }
}

impl<D: MaybeDialect, V: MaybeVersioned> Frame<D, V> {
    /// Returns wrapped [`CoreFrame`].
    pub fn frame(&self) -> &CoreFrame<V> {
        &self.frame
    }

    /// MAVLink protocol version.
    ///
    /// See [`CoreFrame::mavlink_version`] for details.
    #[inline]
    pub fn mavlink_version(&self) -> MavLinkVersion {
        self.frame.mavlink_version()
    }

    /// Payload length.
    ///
    /// See [`CoreFrame::payload_length`] for details.
    #[inline]
    pub fn payload_length(&self) -> u8 {
        self.frame.payload_length()
    }

    /// Packet sequence number.
    ///
    /// See [`CoreFrame::sequence`] for details.
    #[inline]
    pub fn sequence(&self) -> u8 {
        self.frame.sequence()
    }

    /// System `ID`.
    ///
    /// See [`CoreFrame::system_id`] for details.
    #[inline]
    pub fn system_id(&self) -> u8 {
        self.frame.system_id()
    }

    /// Component `ID`.
    ///
    /// See [`CoreFrame::component_id`] for details.
    #[inline]
    pub fn component_id(&self) -> u8 {
        self.frame.component_id()
    }

    /// Message `ID`.
    ///
    /// See [`CoreFrame::message_id`] for details.
    #[inline]
    pub fn message_id(&self) -> MessageId {
        self.frame.message_id()
    }

    /// Payload data.
    ///
    /// See [`CoreFrame::payload`] for details.
    #[inline]
    pub fn payload(&self) -> &Payload {
        self.frame.payload()
    }

    /// MAVLink packet checksum.
    ///
    /// See [`CoreFrame::checksum`] for details.
    #[inline]
    pub fn checksum(&self) -> Checksum {
        self.frame.checksum()
    }
}

impl<D: MaybeDialect> Frame<D, V2> {
    /// Incompatibility flags for `MAVLink 2` frames.
    ///
    /// See [`CoreFrame::incompat_flags`] for details.
    #[inline]
    pub fn incompat_flags(&self) -> IncompatFlags {
        self.frame.incompat_flags()
    }

    /// Compatibility flags for `MAVLink 2` frames.
    ///
    /// See [`CoreFrame::compat_flags`] for details.
    #[inline]
    pub fn compat_flags(&self) -> CompatFlags {
        self.frame.compat_flags()
    }

    /// `MAVLink 2` signature.
    ///
    /// See [`CoreFrame::signature`] for details.
    #[inline]
    pub fn signature(&self) -> Option<&Signature> {
        self.frame.signature()
    }

    /// `MAVLink 2` signature `link_id`, an 8-bit identifier of a MAVLink channel.
    ///
    /// See [`CoreFrame::link_id`] for details.
    #[inline]
    pub fn link_id(&self) -> Option<SignatureLinkId> {
        self.frame.link_id()
    }

    /// `MAVLink 2` signature [`MavTimestamp`], a 48-bit value that specifies the moment when message was sent.
    ///
    /// See [`CoreFrame::timestamp`] for details.
    #[inline]
    pub fn timestamp(&self) -> Option<MavTimestamp> {
        self.frame.timestamp()
    }

    /// Whether a frame is signed.
    ///
    /// See [`CoreFrame::is_signed`] for details.
    #[inline]
    pub fn is_signed(&self) -> bool {
        self.frame.is_signed()
    }
}

/// Builder for [`Frame`].
pub struct FrameBuilder<D: MaybeDialect, V: MaybeVersioned> {
    dialect: D,
    version: V,
}

impl Default for FrameBuilder<Dialectless, Versionless> {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameBuilder<Dialectless, Versionless> {
    /// Create an empty instance of [`FrameBuilder`].
    pub fn new() -> Self {
        Self {
            dialect: Dialectless,
            version: Versionless,
        }
    }
}

impl<V: MaybeVersioned> FrameBuilder<Dialectless, V> {
    /// Defines a MAVLink dialect.
    pub fn dialect<M: DialectMessage + 'static>(
        self,
        dialect: &'static dyn DialectImpl<Message = M>,
    ) -> FrameBuilder<HasDialect<M>, V> {
        FrameBuilder {
            dialect: HasDialect(dialect),
            version: self.version,
        }
    }

    pub(crate) fn dialect_generic<D: MaybeDialect>(self, dialect: D) -> FrameBuilder<D, V> {
        FrameBuilder {
            dialect,
            version: self.version,
        }
    }
}

impl<D: MaybeDialect> FrameBuilder<D, Versionless> {
    /// Restricts this frame to `MAVLink 1` dialect.
    pub fn v1(self) -> FrameBuilder<D, V1> {
        FrameBuilder {
            dialect: self.dialect,
            version: V1,
        }
    }

    /// Restricts this frame to `MAVLink 2` dialect.
    pub fn v2(self) -> FrameBuilder<D, V2> {
        FrameBuilder {
            dialect: self.dialect,
            version: V2,
        }
    }

    pub(crate) fn version_generic<V: MaybeVersioned>(self, version: V) -> FrameBuilder<D, V> {
        FrameBuilder {
            dialect: self.dialect,
            version,
        }
    }
}

impl<D: MaybeDialect, V: MaybeVersioned> FrameBuilder<D, V> {
    /// Builds an instance of [`Frame`].
    pub fn build_for(self, frame: CoreFrame<V>) -> Result<Frame<D, V>> {
        V::expect(frame.mavlink_version())?;
        self.dialect.matches_frame(frame.message_id())?;

        Ok(Frame {
            frame,
            dialect: self.dialect,
        })
    }
}
