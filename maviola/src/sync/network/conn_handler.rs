use std::collections::HashMap;
use std::sync::mpsc;
use std::thread;
use std::thread::JoinHandle;

use crate::core::consts::NETWORK_POOLING_INTERVAL;
use crate::core::io::{ConnectionInfo, IncomingFrame, Retry};
use crate::core::marker::Proxy;
use crate::core::network::types::{NetworkConnInfo, NetworkConnState, RestartNodeEvent};
use crate::core::node::NodeConf;
use crate::core::utils::{Closer, UniqueId};
use crate::error::{NodeError, RecvTimeoutError};
use crate::sync::io::{ChannelFactory, IncomingFrameProducer, OutgoingFrameHandler};
use crate::sync::marker::ConnConf;

use crate::prelude::*;
use crate::sync::prelude::*;

/// Manages the entire [`Network`] connection.
pub(super) struct NetworkConnectionHandler<V: MaybeVersioned> {
    state: Closer,
    info: ConnectionInfo,
    retry: Retry,
    stop_on_node_down: bool,
    node_configs: HashMap<UniqueId, NodeConf<Proxy, V, ConnConf<V>>>,
    nodes: HashMap<UniqueId, Node<Proxy, V, SyncApi<V>>>,
    producer: IncomingFrameProducer<V>,
    send_handler: OutgoingFrameHandler<V>,
    closed_nodes_chan: ClosedNodesChannel,
    node_events_chan: RestartEventsChannel<V>,
}

/// Handles incoming events of a particular [`Node`] withing a [`Network`].
struct IncomingEventsHandler<V: MaybeVersioned> {
    id: UniqueId,
    info: NetworkConnInfo,
    state: NetworkConnState,
    receiver: EventReceiver<V>,
    producer: IncomingFrameProducer<V>,
}

/// Handles outgoing frames of a particular [`Node`] withing a [`Network`].
struct OutgoingFramesHandler<V: MaybeVersioned> {
    id: UniqueId,
    info: NetworkConnInfo,
    state: NetworkConnState,
    send_handler: OutgoingFrameHandler<V>,
    sender: FrameSender<V, Proxy>,
}

/// Manages the state of a particular [`Node`] withing a [`Network`].
struct NodeStateHandler {
    id: UniqueId,
    info: NetworkConnInfo,
    state: NetworkConnState,
    in_handler: JoinHandle<UniqueId>,
    out_handler: JoinHandle<UniqueId>,
    on_close_tx: mpsc::Sender<UniqueId>,
}

/// Channel, that communicate closed [`Node`] events.
struct ClosedNodesChannel {
    tx: mpsc::Sender<UniqueId>,
    rx: mpsc::Receiver<UniqueId>,
}

/// Channel, that communicates [`Node`] restart events.
struct RestartEventsChannel<V: MaybeVersioned> {
    tx: mpsc::Sender<RestartNodeEvent<V, SyncApi<V>>>,
    rx: mpsc::Receiver<RestartNodeEvent<V, SyncApi<V>>>,
}

impl<V: MaybeVersioned> NetworkConnectionHandler<V> {
    pub(super) fn new(
        state: Closer,
        network: &Network<V, ConnConf<V>>,
        chan_factory: ChannelFactory<V>,
    ) -> Result<Self> {
        let node_configs = network.nodes.clone();
        let mut nodes = HashMap::new();

        for (id, node_conf) in &node_configs {
            let node = node_conf.clone().build()?;

            nodes.insert(*id, node);
        }

        Ok(Self {
            state,
            info: network.info.clone(),
            retry: network.retry,
            stop_on_node_down: network.stop_on_node_down,
            node_configs,
            nodes,
            producer: chan_factory.producer().clone(),
            send_handler: chan_factory.send_handler().clone(),
            closed_nodes_chan: ClosedNodesChannel::new(),
            node_events_chan: RestartEventsChannel::synchronous(),
        })
    }

    pub(super) fn handle(mut self) -> Result<()> {
        let state = self.state.to_closable();
        let info = self.info.clone();

        for (id, node) in &self.nodes {
            self.spawn_node_handlers(*id, node, self.closed_nodes_chan.tx.clone())?;
        }

        while !state.is_closed() {
            if let Ok(event) = self.node_events_chan.rx.try_recv() {
                match event {
                    RestartNodeEvent::New(id, node) => {
                        self.nodes.insert(id, node);
                    }
                    RestartNodeEvent::Retry(id, strategy) => {
                        if self.on_node_restart_retry(id, strategy).is_err() {
                            break;
                        }
                    }
                    RestartNodeEvent::GiveUp(id) => {
                        if self.on_node_give_up(id).is_err() {
                            break;
                        }
                    }
                }
            }

            match self
                .closed_nodes_chan
                .rx
                .recv_timeout(NETWORK_POOLING_INTERVAL)
            {
                Ok(id) => {
                    self.nodes.remove(&id);
                    if let Err(err) = self.on_node_stopped(id) {
                        log::error!("[{info:?}] can't process node stop event: {err:?}");
                        break;
                    };
                }
                Err(err) => {
                    if err == mpsc::RecvTimeoutError::Disconnected {
                        break;
                    }
                }
            };

            if self.node_configs.is_empty() {
                break;
            }
        }

        log::info!("[{info:?}] main handler stopped");
        Ok(())
    }

    fn on_node_stopped(&self, id: UniqueId) -> Result<()> {
        if let Some(node_conf) = self.node_configs.get(&id) {
            let conn_info = node_conf.connection_conf.0.info();
            log::info!("[{:?}] node {conn_info:?} stopped", &self.info);

            if node_conf.is_repairable() {
                let tx = self.node_events_chan.tx.clone();

                match self.retry {
                    Retry::Never => {
                        self.node_events_chan
                            .tx
                            .send(RestartNodeEvent::GiveUp(id))?;
                    }
                    Retry::Attempts(attempts, interval) => {
                        thread::spawn(move || {
                            thread::sleep(interval);
                            tx.send(RestartNodeEvent::Retry(
                                id,
                                Retry::Attempts(attempts, interval),
                            ))
                            .unwrap();
                        });
                    }
                    Retry::Always(interval) => {
                        thread::spawn(move || {
                            thread::sleep(interval);
                            tx.send(RestartNodeEvent::Retry(id, Retry::Always(interval)))
                                .unwrap();
                        });
                    }
                }
            } else {
                self.node_events_chan
                    .tx
                    .send(RestartNodeEvent::GiveUp(id))?;
            }
        }

        Ok(())
    }

    fn on_node_restart_retry(&self, id: UniqueId, retry: Retry) -> Result<()> {
        if let Retry::Never = retry {
            self.node_events_chan
                .tx
                .send(RestartNodeEvent::GiveUp(id))?;
            return Ok(());
        }

        let node_conf = if let Some(node_conf) = self.node_configs.get(&id) {
            node_conf
        } else {
            return Ok(());
        };
        let conn_info = node_conf.connection_conf.0.info();
        log::debug!(
            "[{:?}] attempting to restart node {conn_info:?}: {retry:?}",
            self.info
        );

        match self.restart_node(id, node_conf) {
            Ok(node) => {
                self.node_events_chan
                    .tx
                    .send(RestartNodeEvent::New(id, node))?;
            }
            Err(_) => {
                let tx = self.node_events_chan.tx.clone();

                match retry {
                    Retry::Attempts(attempts, _) if attempts <= 1 => {
                        log::debug!(
                            "[{:?}] no restart attempts left for node {conn_info:?}, giving up",
                            self.info
                        );
                        self.node_events_chan
                            .tx
                            .send(RestartNodeEvent::GiveUp(id))?;
                    }
                    Retry::Attempts(attempts, interval) => {
                        thread::spawn(move || {
                            thread::sleep(interval);
                            tx.send(RestartNodeEvent::Retry(
                                id,
                                Retry::Attempts(attempts - 1, interval),
                            ))
                            .unwrap();
                        });
                    }
                    Retry::Always(interval) => {
                        thread::spawn(move || {
                            thread::sleep(interval);
                            tx.send(RestartNodeEvent::Retry(id, Retry::Always(interval)))
                                .unwrap();
                        });
                    }
                    Retry::Never => unreachable!(),
                }
            }
        }

        Ok(())
    }

    fn on_node_give_up(&mut self, id: UniqueId) -> Result<()> {
        if let Some(conf) = self.node_configs.get(&id) {
            log::info!(
                "[{:?}] give up node {:?}",
                self.info,
                conf.connection().info()
            );
        }
        self.node_configs.remove(&id);

        if self.stop_on_node_down {
            return Err(Error::from(NodeError::Inactive));
        }

        Ok(())
    }

    fn restart_node(
        &self,
        id: UniqueId,
        node_conf: &NodeConf<Proxy, V, ConnConf<V>>,
    ) -> Result<Node<Proxy, V, SyncApi<V>>> {
        let conn_info = node_conf.connection_conf.0.info();

        if node_conf.is_repairable() {
            let node = node_conf.clone().build()?;
            self.spawn_node_handlers(id, &node, self.closed_nodes_chan.tx.clone())?;
            log::info!("[{:?}] node {conn_info:?} restarted", self.info);
            return Ok(node);
        } else {
            log::warn!(
                "[{:?}] attempt to restart non-repairable node {conn_info:?}",
                self.info
            );
        }

        Err(Error::Node(NodeError::Inactive))
    }

    fn spawn_node_handlers(
        &self,
        id: UniqueId,
        node: &Node<Proxy, V, SyncApi<V>>,
        on_close_tx: mpsc::Sender<UniqueId>,
    ) -> Result<()> {
        let info = NetworkConnInfo {
            network: self.info.clone(),
            connection: node.info().clone(),
        };
        let state = NetworkConnState {
            network: self.state.to_closable(),
            connection: node.state.clone(),
        };

        let in_handler = IncomingEventsHandler {
            id,
            info: info.clone(),
            state: state.clone(),
            receiver: node.receiver().clone(),
            producer: self.producer.clone(),
        }
        .spawn();

        let out_handler = OutgoingFramesHandler {
            id,
            info: info.clone(),
            state: state.clone(),
            send_handler: self.send_handler.clone(),
            sender: node.frame_sender().clone(),
        }
        .spawn();

        NodeStateHandler {
            id,
            info: info.clone(),
            state: state.clone(),
            in_handler,
            out_handler,
            on_close_tx,
        }
        .spawn();

        Ok(())
    }
}

impl<V: MaybeVersioned> IncomingEventsHandler<V> {
    /// Spawns incoming events handler.
    fn spawn(self) -> JoinHandle<UniqueId> {
        thread::spawn(move || {
            let id = self.id;
            let info = self.info.clone();

            if let Err(err) = self.handle() {
                log::warn!("[{info}] incoming frames handler exited with error: {err:?}");
            }

            id
        })
    }

    /// Handles incoming events.
    fn handle(self) -> Result<()> {
        let state = self.state.clone();

        while !state.is_closed() {
            let (frame, callback) = match self.receiver.recv_timeout(NETWORK_POOLING_INTERVAL) {
                Ok(event) => match event {
                    Event::Frame(frame, callback) => (frame, callback),
                    _ => continue,
                },
                Err(err) => match err {
                    RecvTimeoutError::Disconnected => break,
                    RecvTimeoutError::Timeout | RecvTimeoutError::Lagged(_) => continue,
                },
            };

            self.producer
                .send(IncomingFrame::new(frame, callback.into()))?;
        }

        Ok(())
    }
}

impl<V: MaybeVersioned> OutgoingFramesHandler<V> {
    /// Spawns outgoing frames handler.
    fn spawn(self) -> JoinHandle<UniqueId> {
        thread::spawn(move || {
            let id = self.id;
            let info = self.info.clone();

            if let Err(err) = self.handle() {
                log::warn!("[{info}] outgoing frames handler exited with error: {err:?}");
            }

            id
        })
    }

    /// Handles outgoing frames.
    fn handle(self) -> Result<()> {
        let state = self.state.clone();

        while !state.is_closed() {
            let mut frame = match self.send_handler.recv_timeout(NETWORK_POOLING_INTERVAL) {
                Ok(value) => value,
                Err(err) => match err {
                    RecvTimeoutError::Disconnected => break,
                    RecvTimeoutError::Timeout | RecvTimeoutError::Lagged(_) => continue,
                },
            };

            if !frame.matches_connection_reroute(self.info.network.id()) {
                continue;
            }

            self.sender.send_raw(frame)?;
        }

        Ok(())
    }
}

impl NodeStateHandler {
    /// Spawns state handler, that monitors nodes state and notifies [`NetworkConnectionHandler`]
    /// when node is down.
    fn spawn(self) {
        let id = self.id;
        let info = self.info.clone();
        let state_change_tx = self.on_close_tx.clone();

        thread::spawn(move || {
            if let Err(err) = self.handle() {
                log::error!("[{info}] stop handler exited with error: {err:?}")
            }

            _ = state_change_tx.send(id);
        });
    }

    /// Waits until node is closed.
    fn handle(self) -> Result<()> {
        while !self.state.is_closed() {
            if self.in_handler.is_finished() || self.out_handler.is_finished() {
                break;
            }

            thread::sleep(NETWORK_POOLING_INTERVAL);
        }

        Ok(())
    }
}

impl ClosedNodesChannel {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self { tx, rx }
    }
}

impl<V: MaybeVersioned> RestartEventsChannel<V> {
    fn synchronous() -> Self {
        let (tx, rx) = mpsc::channel();
        Self { tx, rx }
    }
}
