//! MAVLink frame.

use std::fmt::{Debug, Formatter};

use mavio::protocol::{
    Checksum, DialectImpl, DialectMessage, MavLinkVersion, MavTimestamp, MessageId, Payload,
    Signature, SignatureLinkId,
};

use crate::prelude::*;

use crate::protocol::variants::{
    HasDialect, IsDialect, IsVersioned, MavLink1, MavLink2, NoDialect, NotVersioned, Versioned,
};

/// MAVLink frame within a specific dialect.
///
/// This is a wrapper around [`mavio::Frame`] that allows to decode MAVLink message in the context
/// of a specific MAVLink dialect.
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Frame<D: IsDialect, V: IsVersioned> {
    frame: mavio::Frame,
    dialect: D,
    version: V,
}

impl Debug for Frame<NoDialect, NotVersioned> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Frame").field("frame", &self.frame).finish()
    }
}

impl<V: Versioned> Debug for Frame<NoDialect, V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let dialect = f.debug_struct("NoDialect").finish();

        f.debug_struct("Frame")
            .field("frame", &self.frame)
            .field("dialect", &dialect)
            .field("version", &self.version.mavlink_version())
            .finish()
    }
}

impl<M: DialectMessage + 'static, V: Versioned> Debug for Frame<HasDialect<M>, V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let dialect = f
            .debug_struct("Dialect")
            .field("name", &self.dialect.0.name())
            .finish_non_exhaustive();

        f.debug_struct("Frame")
            .field("frame", &self.frame)
            .field("dialect", &dialect)
            .field("version", &self.version.mavlink_version())
            .finish()
    }
}

impl<M: DialectMessage + 'static> Debug for Frame<HasDialect<M>, NotVersioned> {
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

impl<D: IsDialect, V: IsVersioned> From<Frame<D, V>> for mavio::Frame {
    /// Converts [`Frame`] into [`mavio::Frame`].
    fn from(value: Frame<D, V>) -> Self {
        value.frame
    }
}

impl Frame<NoDialect, NotVersioned> {
    /// Create an instance of [`Frame`] without neither a specified dialect, nor MAVLink protocol
    /// version.
    ///
    /// Use [`Frame::builder`] to create construct frames with additional restrictions.
    pub fn new(frame: mavio::Frame) -> Self {
        Self {
            frame,
            dialect: NoDialect(),
            version: NotVersioned(),
        }
    }

    /// Instantiate an empty instance of a [`FrameBuilder`].
    pub fn builder() -> FrameBuilder<NoDialect, NotVersioned> {
        FrameBuilder::new()
    }
}

impl<M: DialectMessage + 'static, V: IsVersioned> Frame<HasDialect<M>, V> {
    /// Decodes MAVLink frame within a specified dialect.
    pub fn decode(&self) -> Result<M> {
        self.dialect
            .0
            .decode(self.frame.payload())
            .map_err(Error::from)
    }
}

impl<D: IsDialect, V: IsVersioned> Frame<D, V> {
    /// Returns wrapped [`mavio::Frame`].
    pub fn frame(&self) -> &mavio::Frame {
        &self.frame
    }

    /// MAVLink protocol version.
    ///
    /// See [`mavio::Frame::mavlink_version`] for details.
    #[inline]
    pub fn mavlink_version(&self) -> MavLinkVersion {
        self.frame.mavlink_version()
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
}

impl<D: IsDialect, V: IsVersioned> IsVersioned for Frame<D, V> {}

impl<D: IsDialect, V: IsVersioned> Versioned for Frame<D, V> {
    /// MAVLink protocol version.
    ///
    /// See [`mavio::Frame::mavlink_version`] for details.
    #[inline]
    fn mavlink_version(&self) -> MavLinkVersion {
        self.frame.mavlink_version()
    }
}

impl<D: IsDialect> Frame<D, MavLink2> {
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

/// Builder for [`Frame`].
pub struct FrameBuilder<D: IsDialect, V: IsVersioned> {
    dialect: D,
    version: V,
}

impl Default for FrameBuilder<NoDialect, NotVersioned> {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameBuilder<NoDialect, NotVersioned> {
    /// Create an empty instance of [`FrameBuilder`].
    pub fn new() -> Self {
        Self {
            dialect: NoDialect(),
            version: NotVersioned(),
        }
    }
}

impl<V: IsVersioned> FrameBuilder<NoDialect, V> {
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

    pub(crate) fn dialect_generic<D: IsDialect>(self, dialect: D) -> FrameBuilder<D, V> {
        FrameBuilder {
            dialect,
            version: self.version,
        }
    }
}

impl<D: IsDialect> FrameBuilder<D, NotVersioned> {
    /// Restricts this frame to `MAVLink 1` dialect.
    pub fn v1(self) -> FrameBuilder<D, MavLink1> {
        FrameBuilder {
            dialect: self.dialect,
            version: MavLink1(),
        }
    }

    /// Restricts this frame to `MAVLink 2` dialect.
    pub fn v2(self) -> FrameBuilder<D, MavLink2> {
        FrameBuilder {
            dialect: self.dialect,
            version: MavLink2(),
        }
    }

    pub(crate) fn version_generic<V: IsVersioned>(self, version: V) -> FrameBuilder<D, V> {
        FrameBuilder {
            dialect: self.dialect,
            version,
        }
    }
}

impl<D: IsDialect, V: IsVersioned> FrameBuilder<D, V> {
    /// Builds an instance of [`Frame`].
    pub fn build_for(self, frame: mavio::Frame) -> Result<Frame<D, V>> {
        self.version.matches_frame(&frame)?;
        self.dialect.matches_frame(&frame)?;

        Ok(Frame {
            frame,
            dialect: self.dialect,
            version: self.version,
        })
    }
}

// impl FrameBuilder<NoDialect, NotVersioned> {
//     fn validate(&self) -> Result<()> {
//         Ok(())
//     }
// }

// impl<D: IsDialect, V: Versioned> FrameBuilder<D, V> {
//     #[inline]
//     fn matches_version(&self, frame: &mavio::Frame) -> Result<()> {
//         if self.version.mavlink_version() != frame.mavlink_version() {
//             return Err(FrameBuildError::InvalidVersion {
//                 frame: frame.clone(),
//                 given: frame.mavlink_version(),
//                 expected: self.version.mavlink_version(),
//             }
//             .into());
//         }
//         Ok(())
//     }
// }

// impl<V: Versioned> FrameBuilder<NoDialect, V> {
//     fn validate(self, frame: &mavio::Frame) -> Result<()> {
//         self.matches_version(&frame)?;
//         Ok(())
//     }
// }
//
// impl<M: DialectMessage + 'static> FrameBuilder<HasDialect<M>, NotVersioned> {
//     fn validate(self, frame: &mavio::Frame) -> Result<()> {
//         self.is_in_dialect(&frame)?;
//         Ok(())
//     }
// }
//
// impl<M: DialectMessage + 'static, V: Versioned> FrameBuilder<HasDialect<M>, V> {
//     fn validate(self, frame: &mavio::Frame) -> Result<()> {
//         self.is_in_dialect(&frame)?;
//         self.matches_version(&frame)?;
//         Ok(())
//     }
// }
