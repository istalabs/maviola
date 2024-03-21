use mavio::protocol::UpdateFrameUnsafe;
use portpicker::{pick_unused_port, Port};
use std::time::Duration;

use maviola::dialects::minimal::messages::Heartbeat;
use maviola::error::FrameError;
use maviola::protocol::{
    Checksum, CrcExtra, Header, MavFrame, ProcessFrame, ProcessFrameCase, Signature, UpdateFrame,
};

use maviola::prelude::*;

const RECV_TIMEOUT: Duration = Duration::from_millis(5);
const HOST: &str = "127.0.0.1";

fn port() -> Port {
    pick_unused_port().unwrap()
}

fn addr(port: Port) -> String {
    format!("{HOST}:{}", port)
}

/// Custom frame processor, that flips bits in payload for known MAVLink messages and rejects
/// unknown messages.
///
/// Internally, this logic is implemented by [`Scrambler`], that implements [`UpdateFrame`] trait.
/// The latter allows low-level access to frame data.
#[derive(Debug, Default)]
struct CustomProcessor {
    updater: Scrambler,
}

/// Flips bits in [`Frame::payload`].
#[derive(Default, Debug)]
struct Scrambler;

impl<V: MaybeVersioned> UpdateFrameUnsafe<V> for Scrambler {
    /// Just flip bits in payload, all other operations will be handled by [`UpdateFrame`] methods.
    unsafe fn update_unsafe(
        &mut self,
        _: Header<V>,
        payload: &mut [u8],
        _: &mut Checksum,
        _: &mut Option<Signature>,
    ) {
        for i in 0..payload.len() {
            payload[i] = payload[i] ^ 0xff;
        }
    }
}

// This trait provides safe interfaces for updating frames
impl<V: MaybeVersioned> UpdateFrame<V> for Scrambler {}

impl ProcessFrame for CustomProcessor {
    /// Applies [`Scrambler`] to all known frames.
    ///
    /// Internally we use [`UpdateFrame::update`], that ensures, that frames will have a correct
    /// checksum.
    ///
    /// In some cases it makes sense to use unsafe [`UpdateFrame::update_unchecked`] to update even
    /// unknown frames for which impossible to calculate a valid checksum, but it doesn't make sense
    /// for the showcase purposes.
    ///
    /// As you can see, here we operate on [`MavFrame`] instead of [`Frame`]. The former is an enum,
    /// that may hold either `MAVLink 1`, or `MAVLink 2` frames.
    fn process(
        &mut self,
        frame: &mut MavFrame,
        case: ProcessFrameCase,
        crc_extra: Option<CrcExtra>,
    ) -> std::result::Result<(), FrameError> {
        // Reject unknown frames
        let crc_extra = match crc_extra {
            None => return Err(FrameError::NotInDialect(frame.message_id())),
            Some(crc_extra) => crc_extra,
        };

        match case {
            // We are going to flip payload bits for outgoing frames after they were processed and
            // for incoming frames before any processing
            ProcessFrameCase::IncomingBefore | ProcessFrameCase::OutgoingAfter => {
                log::info!("[scrambler] payload before: {:?}", frame.payload().bytes());
                match frame {
                    MavFrame::V1(frame) => self.updater.update(frame, crc_extra),
                    MavFrame::V2(frame) => self.updater.update(frame, crc_extra),
                }
                log::info!("[scrambler] payload after: {:?}", frame.payload().bytes());
            }
            _ => {}
        }
        Ok(())
    }
}

fn run(addr: &str) -> Result<()> {
    // A server that knows, how to scramble and unscramble messages
    let server = Node::sync::<V2>()
        .id(MavLinkId::new(1, 0))
        .connection(TcpServer::new(addr)?)
        .add_processor("scrambler", CustomProcessor::default())
        .build()?;
    // A client that knows, how to scramble and unscramble messages
    let secure_client = Node::sync::<V2>()
        .id(MavLinkId::new(1, 0))
        .connection(TcpClient::new(addr)?)
        .add_processor("scrambler", CustomProcessor::default())
        .build()?;
    // A regular client, that knows nothing about scrambling algorithm and will fail to decode
    // messages
    let unsecure_client = Node::sync::<V2>()
        .id(MavLinkId::new(1, 0))
        .connection(TcpClient::new(addr)?)
        .build()?;

    // Send a message with data we can check later
    server.send(&Heartbeat {
        type_: Default::default(),
        autopilot: Default::default(),
        base_mode: Default::default(),
        custom_mode: 11,
        system_status: Default::default(),
        mavlink_version: 17,
    })?;

    // Receive frame on secure client
    let (frame, _) = secure_client.recv_frame_timeout(RECV_TIMEOUT)?;

    // Validate, that we indeed have a valid message
    if let DefaultDialect::Heartbeat(heartbeat) = frame.decode::<DefaultDialect>()? {
        assert_eq!(heartbeat.custom_mode, 11);
        assert_eq!(heartbeat.mavlink_version, 17);
    } else {
        panic!("invalid frame!")
    }

    // Receive frame on unsecure client
    let (frame, _) = unsecure_client.recv_frame_timeout(RECV_TIMEOUT)?;
    // Validate, that we receive a frame with a correct checksum
    frame.validate_checksum::<DefaultDialect>()?;
    // But the contents of a frame is a complete junk
    assert!(frame.decode::<DefaultDialect>().is_err());

    Ok(())
}

fn main() {
    // Setup logger
    env_logger::builder()
        .filter_level(log::LevelFilter::Info) // Suppress everything below `info` for third-party modules.
        .filter_module(env!("CARGO_PKG_NAME"), log::LevelFilter::Info) // Log level for current package
        .init();

    let addr = addr(port());
    run(addr.as_str()).unwrap();
}

#[cfg(test)]
#[test]
fn scrambler() {
    use std::thread;

    let addr = addr(port());
    let handler = thread::spawn(move || {
        run(addr.as_str()).unwrap();
    });

    for _ in 0..10 {
        thread::sleep(Duration::from_millis(250));
        if handler.is_finished() {
            handler.join().unwrap();
            return;
        }
    }

    if !handler.is_finished() {
        panic!("[scrambler] test took too long")
    }
}
