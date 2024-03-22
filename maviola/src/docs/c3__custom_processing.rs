/*!
# 📖 3.3. Custom Processing

<em>[← Custom Transport](crate::docs::c2__custom_transport) | [Ad-hoc Dialects →](crate::docs::c4__ad_hoc_dialects)</em>

Maviola allows to add custom frame processors to nodes. These processors should implement
[`ProcessFrame`] frame trait and have access to internal frame state.

This part of the API is considered dangerous and is available only under the `unsafe` Cargo feature
flag.

## Making A Scrambler

This part of the documentation is still under development. For now, let's just consider a simple
example. This will create a custom frame processor that flips bits of a frame:

```rust,no_run
use maviola::prelude::*;
use maviola::protocol::*;
use maviola::error::FrameError;

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

// Now we can use our CustomProcessor in node configuration
let node = Node::sync::<V2>()
    .add_processor("scrambler", CustomProcessor::default())
    /* other node setting */
    # .id(MavLinkId::new(1, 17))
    # .connection(TcpClient::new("127.0.0.1:5600").unwrap())
    .build().unwrap();
```

Our node will scramble outgoing frames and unscramble incoming frames. Only nodes with the same
processor will be able to communicate with our node. Definitely not secure. But you've got the idea.

<em>[← Custom Transport](crate::docs::c2__custom_transport) | [Ad-hoc Dialects →](crate::docs::c4__ad_hoc_dialects)</em>
 */

#[cfg(doc)]
use crate::prelude::*;
#[cfg(doc)]
use crate::protocol::*;
