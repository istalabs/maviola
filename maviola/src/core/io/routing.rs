use crate::core::io::ChannelInfo;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

use crate::core::utils::UniqueId;
use crate::protocol::{Frame, MaybeVersioned};

/// Connection `ID`.
///
/// Identifies a particular connection.
///
/// This is an opaque identifier. It can be compared for equality with other connection `ID` and
/// used as a key in hashmaps or hashsets.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct ConnectionId(UniqueId);

/// Channel `ID`.
///
/// Identifies a channel within a particular connection.
///
/// This is an opaque identifier. It can be compared for equality with other channel `ID` and
/// used as a key in hashmaps or hashsets.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct ChannelId {
    connection: ConnectionId,
    channel: UniqueId,
}

/// Incoming MAVLink frame.
#[derive(Clone, Debug)]
pub struct IncomingFrame<V: MaybeVersioned> {
    frame: Frame<V>,
    channel: ChannelInfo,
}

/// Outgoing MAVLink frame.
#[derive(Clone, Debug)]
pub struct OutgoingFrame<V: MaybeVersioned> {
    frame: Arc<Frame<V>>,
    scope: BroadcastScope,
}

/// Defines, how frame should be broadcast.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum BroadcastScope {
    /// Broadcast to all channels (default value).
    #[default]
    All,
    /// Broadcast only to this channel.
    ExactChannel(ChannelId),
    /// Broadcast to all channels except the specified one.
    ExceptChannel(ChannelId),
    /// Broadcast to all channels of its own connection except the specified channel.
    ExceptChannelWithin(ChannelId),
    /// Broadcast only to this connection.
    ExactConnection(ConnectionId),
    /// Broadcast to all connections except the specified one.
    ExceptConnection(ConnectionId),
}

impl ConnectionId {
    /// Creates a new unique connection identifier.
    pub(crate) fn new() -> Self {
        Self(UniqueId::new())
    }

    /// Returns `true` if the channel with provided `channel_id` belongs to this connection.
    #[inline(always)]
    pub fn contains(&self, channel_id: &ChannelId) -> bool {
        &channel_id.connection == self
    }
}

impl ChannelId {
    /// Creates a new unique channel identifier that belongs to the connection with the specified
    /// `connection_id`.
    pub(crate) fn new(connection_id: ConnectionId) -> Self {
        Self {
            connection: connection_id,
            channel: UniqueId::new(),
        }
    }

    /// Identifier of a connection withing this channel.
    pub fn connection_id(&self) -> &ConnectionId {
        &self.connection
    }

    /// Returns `true`, if this channel belongs to the connection with provided `connection_id`.
    #[inline(always)]
    pub fn belongs_to(&self, connection_id: &ConnectionId) -> bool {
        connection_id.contains(self)
    }
}

impl Debug for ConnectionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ConnectionId").finish()
    }
}

impl Debug for ChannelId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ChannelId").finish()
    }
}

impl From<ConnectionId> for ChannelId {
    fn from(value: ConnectionId) -> Self {
        Self::new(value)
    }
}

impl<V: MaybeVersioned> IncomingFrame<V> {
    /// Creates an incoming from MAVLink [`Frame`] and [`ChannelId`].
    pub fn new(frame: Frame<V>, channel: ChannelInfo) -> Self {
        Self { frame, channel }
    }
}

impl<V: MaybeVersioned> From<IncomingFrame<V>> for (Frame<V>, ChannelInfo) {
    fn from(value: IncomingFrame<V>) -> Self {
        (value.frame, value.channel)
    }
}

impl<V: MaybeVersioned> OutgoingFrame<V> {
    /// Creates an outgoing frame from MAVLink [`Frame`].
    pub fn new(frame: Frame<V>) -> Self {
        Self {
            frame: Arc::new(frame),
            scope: BroadcastScope::All,
        }
    }

    pub(crate) fn scoped(frame: Frame<V>, scope: BroadcastScope) -> Self {
        Self {
            frame: Arc::new(frame),
            scope,
        }
    }

    /// Reference to the underlying MAVLink [`Frame`].
    #[inline]
    pub fn frame(&self) -> &Frame<V> {
        self.frame.as_ref()
    }

    /// Broadcast scope.
    #[inline]
    pub fn scope(&self) -> &BroadcastScope {
        &self.scope
    }

    /// Set broadcast scope.
    #[inline]
    pub(crate) fn set_scope(&mut self, scope: BroadcastScope) {
        self.scope = scope;
    }

    /// Matches frame against a particular connection and changes broadcast scope if necessary.
    ///
    /// The rules are the following:
    ///
    /// * If scope is [`BroadcastScope::ExceptConnection`] and `connection_id` equals the excluded
    ///   `ID`, then method will keep connection untouched and return `false`.
    /// * If scope is [`BroadcastScope::ExactConnection`] and `connection_id` equals the specified
    ///   `ID`, then frame scope will be changed to [`BroadcastScope::All`] and `true` will be
    ///   returned.
    /// * Returns `true` for all other cases.
    pub(crate) fn matches_connection_reroute(&mut self, connection_id: &ConnectionId) -> bool {
        match self.scope() {
            BroadcastScope::ExceptConnection(conn_id) if connection_id == conn_id => false,
            BroadcastScope::ExactConnection(conn_id) if connection_id == conn_id => {
                self.set_scope(BroadcastScope::All);
                true
            }
            _ => true,
        }
    }

    pub(crate) fn should_send_to(&self, channel_id: &ChannelId) -> bool {
        match &self.scope {
            BroadcastScope::All => true,
            BroadcastScope::ExactChannel(sender_id) => sender_id == channel_id,
            BroadcastScope::ExceptChannel(sender_id) => sender_id != channel_id,
            BroadcastScope::ExceptChannelWithin(sender_id) => {
                channel_id.connection_id().contains(sender_id) && sender_id != channel_id
            }
            BroadcastScope::ExactConnection(conn_id) => conn_id.contains(channel_id),
            BroadcastScope::ExceptConnection(conn_id) => !conn_id.contains(channel_id),
        }
    }
}

impl<V: MaybeVersioned> From<OutgoingFrame<V>> for Frame<V> {
    fn from(value: OutgoingFrame<V>) -> Self {
        value.frame.as_ref().clone()
    }
}
