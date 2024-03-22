use std::sync::Arc;

use crate::asnc::io::OutgoingFrameSender;
use crate::core::io::{BroadcastScope, OutgoingFrame};
use crate::core::marker::{Edge, NodeKind, Proxy};
use crate::core::node::{SendFrameInternal, SendMessageInternal};
use crate::core::utils::Sealed;
use crate::error::SendResult;
use crate::protocol::FrameProcessor;

use crate::prelude::*;

/// <sup>[`async`](crate::asnc)</sup>
/// Frame sender for asynchronous API.
///
/// **⚠** [`FrameSender`] requires [`SendFrame`], [`SendMessage`], and [`SendVersionlessMessage`]
/// traits to be imported in order to work correctly. The latter two traits make sense only for
/// frame senders exposed by [`Edge`] nodes (like [`EdgeNode`]).
///
/// [`EdgeNode`]: crate::asnc::node::EdgeNode
#[derive(Clone, Debug)]
pub struct FrameSender<V: MaybeVersioned, K: NodeKind> {
    inner: OutgoingFrameSender<V>,
    processor: Arc<FrameProcessor>,
    kind: K,
}

impl<V: MaybeVersioned> FrameSender<V, Proxy> {
    /// <sup>⛔</sup>
    /// Creates a new proxy frame sender.
    pub(super) fn new(sender: OutgoingFrameSender<V>, processor: Arc<FrameProcessor>) -> Self {
        Self {
            inner: sender,
            processor,
            kind: Proxy,
        }
    }

    /// <sup>⛔</sup>
    /// Converts proxy frame sender into edge frame sender.
    pub(super) fn into_edge(self, kind: Edge<V>) -> FrameSender<V, Edge<V>> {
        FrameSender {
            inner: self.inner,
            processor: self.processor,
            kind,
        }
    }
}

impl<V: MaybeVersioned, K: NodeKind> FrameSender<V, K> {
    /// <sup>⛔</sup>
    /// Sends outgoing frame without processing.
    pub(in crate::asnc) unsafe fn send_raw(
        &self,
        frame: OutgoingFrame<V>,
    ) -> SendResult<OutgoingFrame<V>> {
        self.inner.send_raw(frame)
    }

    /// <sup>⛔</sup>
    /// Return a reference to internal frame processor.
    pub(in crate::asnc) fn processor(&self) -> &FrameProcessor {
        self.processor.as_ref()
    }

    /// <sup>⛔</sup>
    /// Sets frame processor.
    pub(in crate::asnc) fn set_processor(&mut self, processor: Arc<FrameProcessor>) {
        self.processor = processor;
    }
}

impl<V: MaybeVersioned, K: NodeKind> Sealed for FrameSender<V, K> {}

impl<V: MaybeVersioned, K: NodeKind> SendFrameInternal<V> for FrameSender<V, K> {
    #[inline(always)]
    fn processor(&self) -> &FrameProcessor {
        self.processor.as_ref()
    }

    #[inline(always)]
    unsafe fn route_frame_internal(&self, frame: Frame<V>, scope: BroadcastScope) -> Result<()> {
        self.inner
            .send_raw(OutgoingFrame::scoped(frame, scope))
            .map_err(Error::from)
    }
}

impl<V: MaybeVersioned, K: NodeKind> SendFrame<V> for FrameSender<V, K> {}

impl<V: MaybeVersioned> SendMessageInternal<V> for FrameSender<V, Edge<V>> {
    fn endpoint(&self) -> &Endpoint<V> {
        &self.kind.endpoint
    }
}

impl<V: Versioned> SendMessage<V> for FrameSender<V, Edge<V>> {}

impl SendVersionlessMessage for FrameSender<Versionless, Edge<Versionless>> {}
