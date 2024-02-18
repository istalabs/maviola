use std::sync::Arc;

use mavio::protocol::MaybeVersioned;
use mavio::Frame;

use crate::io::PeerConnectionInfo;
use crate::utils::UniqueId;

use crate::prelude::*;

/// Synchronous response object which caller receives upon each incoming frame.
#[derive(Clone, Debug)]
pub struct Response<V: MaybeVersioned> {
    pub(crate) sender_id: UniqueId,
    pub(crate) sender_info: Arc<PeerConnectionInfo>,
    pub(crate) broadcast_tx: mpmc::Sender<ResponseFrame<V>>,
}

impl<V: MaybeVersioned> Response<V> {
    /// Information about sender's connection.
    pub fn info(&self) -> &PeerConnectionInfo {
        self.sender_info.as_ref()
    }

    /// Respond directly to the peer which has sent the [`Response`].
    pub fn respond(&self, frame: &Frame<V>) -> Result<()> {
        let frame = Arc::new(frame.clone());
        self.broadcast_tx
            .send(ResponseFrame {
                frame,
                scope: Some(BroadcastScope::Exact(self.sender_id)),
            })
            .map_err(Error::from)
    }

    /// Respond to all the recipients except the one which has sent the initial frame.
    pub fn respond_others(&self, frame: &Frame<V>) -> Result<()> {
        let frame = Arc::new(frame.clone());
        self.broadcast_tx
            .send(ResponseFrame {
                frame,
                scope: Some(BroadcastScope::Except(self.sender_id)),
            })
            .map_err(Error::from)
    }

    /// Respond to all the recipients including the one which has sent the initial.
    pub fn respond_all(&self, frame: &Frame<V>) -> Result<()> {
        let frame = Arc::new(frame.clone());
        self.broadcast_tx
            .send(ResponseFrame {
                frame,
                scope: Some(BroadcastScope::All),
            })
            .map_err(Error::from)
    }
}

///////////////////////////////////////////////////////////////////////////////
//                                 PRIVATE                                   //
///////////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum BroadcastScope {
    All,
    Except(UniqueId),
    Exact(UniqueId),
}

#[derive(Clone, Debug)]
pub(crate) struct ResponseFrame<V: MaybeVersioned> {
    pub(crate) frame: Arc<Frame<V>>,
    pub(crate) scope: Option<BroadcastScope>,
}
