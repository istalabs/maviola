//! MAVLink frame.

use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

use mavio::protocol::{
    Checksum, DialectImpl, DialectMessage, MavLinkVersion, MavTimestamp, MessageId, Payload,
    Signature, SignatureLinkId,
};

use crate::prelude::*;

use crate::io::node_variants::{Dialect, HasDialect, NoDialect};

/// MAVLink frame within a specific dialect.
///
/// This is a wrapper around [`mavio::Frame`] that allows to decode MAVLink message in the context
/// of a specific MAVLink dialect.
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Frame<M: DialectMessage + 'static, D: HasDialect> {
    frame: mavio::Frame,
    dialect: D,
    _marker_message: PhantomData<M>,
}

impl<M: DialectMessage + 'static> Debug for Frame<M, NoDialect> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let dialect = f.debug_struct("NoDialect").finish();

        f.debug_struct("Frame")
            .field("frame", &self.frame)
            .field("dialect", &dialect)
            .finish()
    }
}

impl<M: DialectMessage + 'static> Debug for Frame<M, Dialect<M>> {
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

impl<M: DialectMessage + 'static, D: HasDialect> From<Frame<M, D>> for mavio::Frame {
    /// Converts [`Frame`] into [`mavio::Frame`].
    fn from(value: Frame<M, D>) -> Self {
        value.frame
    }
}

impl<M: DialectMessage + 'static> Frame<M, NoDialect> {
    /// Instantiate a new [`Frame`] without a dialect.
    pub fn new(frame: mavio::Frame) -> Self {
        Self {
            frame,
            dialect: NoDialect(),
            _marker_message: PhantomData,
        }
    }
}

impl<M: DialectMessage + 'static> Frame<M, Dialect<M>> {
    /// Instantiate a new [`Frame`] with a specified `dialect`.
    pub fn with_dialect(
        frame: mavio::Frame,
        dialect: &'static dyn DialectImpl<Message = M>,
    ) -> Self {
        Self {
            frame,
            dialect: Dialect(dialect),
            _marker_message: PhantomData,
        }
    }

    /// Decodes MAVLink frame within a specified dialect.
    pub fn decode(&self) -> Result<M> {
        self.dialect
            .0
            .decode(self.frame.payload())
            .map_err(Error::from)
    }
}

impl<M: DialectMessage + 'static, D: HasDialect> Frame<M, D> {
    /// Returns wrapped [`mavio::Frame`].
    pub fn frame(&self) -> &mavio::Frame {
        &self.frame
    }

    /// MAVLink header.
    ///
    /// See [`mavio::Frame::header`] for details.
    #[inline]
    pub fn header(&self) -> &mavio::protocol::Header {
        self.frame.header()
    }

    /// MAVLink protocol version.
    ///
    /// See [`mavio::Frame::mavlink_version`] for details.
    #[inline]
    pub fn mavlink_version(&self) -> MavLinkVersion {
        self.frame.mavlink_version()
    }

    /// Incompatibility flags for `MAVLink 2` frames.
    ///
    /// See [`mavio::Frame::incompat_flags`] for details.
    #[inline]
    pub fn incompat_flags(&self) -> Option<u8> {
        self.frame.incompat_flags()
    }

    /// Compatibility flags for `MAVLink 2` frames.
    ///
    /// See [`mavio::Frame::compat_flags`] for details.
    #[inline]
    pub fn compat_flags(&self) -> Option<u8> {
        self.frame.compat_flags()
    }

    /// Payload length.
    ///
    /// See [`mavio::Frame::payload_length`] for details.
    #[inline]
    pub fn payload_length(&self) -> u8 {
        self.frame.payload_length()
    }

    /// Packet sequence number.
    ///
    /// See [`mavio::Frame::sequence`] for details.
    #[inline]
    pub fn sequence(&self) -> u8 {
        self.frame.sequence()
    }

    /// System `ID`.
    ///
    /// See [`mavio::Frame::system_id`] for details.
    #[inline]
    pub fn system_id(&self) -> u8 {
        self.frame.system_id()
    }

    /// Component `ID`.
    ///
    /// See [`mavio::Frame::component_id`] for details.
    #[inline]
    pub fn component_id(&self) -> u8 {
        self.frame.component_id()
    }

    /// Message `ID`.
    ///
    /// See [`mavio::Frame::message_id`] for details.
    #[inline]
    pub fn message_id(&self) -> MessageId {
        self.frame.message_id()
    }

    /// Payload data.
    ///
    /// See [`mavio::Frame::payload`] for details.
    #[inline]
    pub fn payload(&self) -> &Payload {
        self.frame.payload()
    }

    /// MAVLink packet checksum.
    ///
    /// See [`mavio::Frame::checksum`] for details.
    #[inline]
    pub fn checksum(&self) -> Checksum {
        self.frame.checksum()
    }

    /// `MAVLink 2` signature.
    ///
    /// See [`mavio::Frame::signature`] for details.
    #[inline]
    pub fn signature(&self) -> Option<&Signature> {
        self.frame.signature()
    }

    /// `MAVLink 2` signature `link_id`, an 8-bit identifier of a MAVLink channel.
    ///
    /// See [`mavio::Frame::link_id`] for details.
    #[inline]
    pub fn link_id(&self) -> Option<SignatureLinkId> {
        self.frame.link_id()
    }

    /// `MAVLink 2` signature [`MavTimestamp`], a 48-bit value that specifies the moment when message was sent.
    ///
    /// See [`mavio::Frame::timestamp`] for details.
    #[inline]
    pub fn timestamp(&self) -> Option<MavTimestamp> {
        self.frame.timestamp()
    }

    /// Whether a frame is signed.
    ///
    /// See [`mavio::Frame::is_signed`] for details.
    #[inline]
    pub fn is_signed(&self) -> bool {
        self.frame.is_signed()
    }
}
