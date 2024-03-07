//! MAVLink [message signing](https://mavlink.io/en/guide/message_signing.html) tools.

use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::atomic::AtomicU64;
use std::sync::{atomic, Arc};
use std::time::SystemTime;

use crate::protocol::{
    MavSha256, MavTimestamp, SecretKey, Sign, SignedLinkId, Signer, SigningConf,
};

use crate::prelude::*;

pub use builder::MessageSignerBuilder;
use builder::{NoLinkId, NoSecretKey};

/// MAVLink [message signing](https://mavlink.io/en/guide/message_signing.html) manager.
///
/// [`MessageSigner`] allows to verify and sign outgoing and incoming frames. There are several
/// signing strategies defined by [`SignStrategy`] which can be specified both for
/// [`MessageSigner::incoming`] and [`MessageSigner::outgoing`] frames.
///
/// Each instance of a manager is configured with the main [`MessageSigner::link_id`] and the main
/// [`MessageSigner::key`]. These will be used to sign frames.
///
/// It is possible to add additional links with corresponding secret keys. These links will be used
/// to validate already signed frames. Depending on a [`SignStrategy`], these frames can be either
/// rejected, kept as they are, or re-signed with the main key and link `ID`. All supported links
/// (including the main one) can be accessed with [`MessageSigner::links`].
///
/// **⚠** Secret keys are excluded from [Serde](https://serde.rs) serialization.
///
/// # Examples
///
/// ```rust
/// use maviola::prelude::*;
///
/// let signer = MessageSigner::builder()
///     .link_id(1)                         // Set main link `ID`
///     .key("main key")                    // Set main key
///     .incoming(SignStrategy::ReSign)     // All incoming frames will be re-signed
///     .outgoing(SignStrategy::Strict)     // Outgoing frames will be signed
///     .add_link(2, "key for the link #2") // Add extra link
///     .build();
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MessageSigner {
    link_id: SignedLinkId,
    incoming: SignStrategy,
    outgoing: SignStrategy,
    #[cfg_attr(feature = "serde", serde(skip_serializing))]
    links: HashMap<SignedLinkId, SecretKey>,
    last_timestamp: UniqueMavTimestamp,
}

/// Message signing strategy.
///
/// Defines how message signing will be applied.
///
/// By default, the [`SignStrategy::Sign`] strategy will be applied.
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SignStrategy {
    /// Apply message signing for all messages. Sign unsigned messages.
    ///
    /// Unsigned messages will be signed. Messages with incorrect signature will be rejected.
    ///
    /// [`Versionless`] frames of `MAVLink 1` protocol will be passed without change.
    #[default]
    Sign,
    /// Enforce message signing for all messages. Re-sign messages with correct signing.
    ///
    /// Unsigned messages will be signed. Messages with incorrect signature will be rejected.
    ///
    /// [`Versionless`] frames of `MAVLink 1` protocol will be passed without change.
    ReSign,
    /// Apply message signing for all messages. Reject messages without signing or with incorrect
    /// signatures.
    ///
    /// Unsigned messages will be rejected. Messages with incorrect signature will be rejected.
    ///
    /// [`Versionless`] frames of `MAVLink 1` protocol will be rejected.
    Strict,
    /// Pass messages as they are.
    Proxy,
    /// Strip any signing information from messages.
    Strip,
}

/// MAVLink timestamp wrapper that preserve uniqueness of timestamp sequence.
///
/// ⚠ It is not strictly guaranteed that the next timestamp will be unique, if multiple clones
/// of a signer are used. Nevertheless, [`UniqueMavTimestamp`] allows to significantly reduc
/// timestamp collisions.
///
/// Used internally by [`MessageSigner`].
#[derive(Clone)]
pub struct UniqueMavTimestamp(Arc<AtomicU64>);

impl MessageSigner {
    /// Creates a [`MessageSigner`] with the main `link_id` / `key` and default strategies.
    ///
    /// # Usage
    ///
    /// ```rust
    /// use maviola::prelude::*;
    ///
    /// let signer = MessageSigner::new(17, "main secret key");
    /// ```
    ///
    /// This is equal to:
    ///
    /// ```rust
    /// use maviola::prelude::*;
    ///
    /// let signer = MessageSigner::builder()
    ///     .link_id(17)
    ///     .key("main secret key")
    ///     .build();
    /// ```
    pub fn new<K: Into<SecretKey>>(link_id: SignedLinkId, key: K) -> Self {
        Self::builder().link_id(link_id).key(key.into()).build()
    }

    /// Instantiates an empty [`MessageSignerBuilder`].
    pub fn builder() -> MessageSignerBuilder<NoLinkId, NoSecretKey> {
        MessageSignerBuilder::new()
    }

    /// Main link `ID`.
    pub fn link_id(&self) -> SignedLinkId {
        self.link_id
    }

    /// Main secret key.
    pub fn key(&self) -> &SecretKey {
        self.links.get(&self.link_id).unwrap()
    }

    /// Signing strategy for incoming messages.
    pub fn incoming(&self) -> SignStrategy {
        self.incoming
    }

    /// Signing strategy for outgoing messages.
    pub fn outgoing(&self) -> SignStrategy {
        self.outgoing
    }

    /// Iterator over supported links.
    ///
    /// Links will always contain the main link `ID` and the main secret key.
    pub fn links(&self) -> impl Iterator<Item = (SignedLinkId, &SecretKey)> {
        self.links.iter().map(|(&link_id, key)| (link_id, key))
    }

    /// Takes incoming frame and processes it according to a [`Self::incoming`] signing strategy.
    #[inline(always)]
    pub fn process_incoming<V: MaybeVersioned>(&self, frame: &mut Frame<V>) -> Result<()> {
        self.process_for_strategy(frame, self.incoming)
    }

    /// Takes outgoing frame and processes it according to a [`Self::outgoing`] signing strategy.
    #[inline(always)]
    pub fn process_outgoing<V: MaybeVersioned>(&self, frame: &mut Frame<V>) -> Result<()> {
        self.process_for_strategy(frame, self.outgoing)
    }

    /// Prepare a frame that is supposed to be sent via this channel.
    ///
    /// This method will sign a frame only if channel is supposed to accept only signed frames.
    #[allow(unused_variables)]
    pub fn process_new<V: MaybeVersioned>(&self, frame: &mut Frame<V>) {
        if let SignStrategy::Strict = self.outgoing {
            self.sign_frame(frame);
        }
    }

    /// Processes a [`Frame`] given the provided [`SignStrategy`].
    ///
    /// Frame will be validated, then its signature will be added, replaced, or stripped based on
    /// the strategy.
    pub fn process_for_strategy<V: MaybeVersioned>(
        &self,
        frame: &mut Frame<V>,
        strategy: SignStrategy,
    ) -> Result<()> {
        self.validate_for_strategy(frame, strategy)?;
        self.sign_for_strategy(frame, strategy);
        Ok(())
    }

    /// Validates a [`Frame`] given the provided [`SignStrategy`].
    pub fn validate_for_strategy<V: MaybeVersioned>(
        &self,
        frame: &Frame<V>,
        strategy: SignStrategy,
    ) -> Result<()> {
        if let SignStrategy::Proxy = strategy {
            return Ok(());
        }

        if let SignStrategy::Strict = strategy {
            if !frame.is_signed() {
                return Err(FrameError::InvalidSignature.into());
            }
        }

        match strategy {
            SignStrategy::Sign | SignStrategy::ReSign | SignStrategy::Strict => {
                if frame.is_signed() && !self.has_valid_signature(frame) {
                    return Err(FrameError::InvalidSignature.into());
                }
            }
            SignStrategy::Proxy | SignStrategy::Strip => {}
        }

        Ok(())
    }

    /// Adds signature to a [`Frame`].
    ///
    /// Adds signature to `MAVLink 2` frames using main key and link `ID`. `MAVLink 2` frames will
    /// be kept untouched.
    pub fn sign_frame<V: MaybeVersioned>(&self, frame: &mut Frame<V>) {
        let signature_conf = self.to_signature_conf();
        signature_conf.apply(frame, &mut self.signer());
    }

    /// Returns `true` if frame has a valid signature.
    ///
    /// Attempts to validate frame signature by searching for a suitable key given the provided
    /// [`Frame::link_id`]. If such link `ID` is not among the available [`MessageSigner::links`],
    /// then frame will be considered invalid.
    ///
    /// Unsigned frames and `MAVLink 1` frames are always invalid.
    pub fn has_valid_signature<V: MaybeVersioned>(&self, frame: &Frame<V>) -> bool {
        let signature = if let Some(signature) = frame.signature() {
            signature
        } else {
            return false;
        };

        if let Some(key) = self.links.get(&signature.link_id) {
            let mut _signer = self.signer();
            let mut signer = Signer::new(&mut _signer);
            return signer.validate(frame, signature, key);
        }

        false
    }

    /// Returns a new instance of a signer that implements [`Sign`].
    pub fn signer(&self) -> impl Sign {
        MavSha256::default()
    }

    /// Creates an instance of a signature configuration that can be used to sign frames.
    pub fn to_signature_conf(&self) -> SigningConf {
        SigningConf {
            link_id: self.link_id,
            timestamp: self.next_timestamp(),
            secret: self.key().clone(),
        }
    }

    /// Returns the next MAVLink timestamp that can be used to sign a frame.
    ///
    /// ⚠ It is not strictly guaranteed that the next timestamp will be unique, if multiple clones
    /// of a signer are used. Nevertheless, this method allows to significantly reduce timestamp
    /// collisions.
    ///
    /// Uses [`UniqueMavTimestamp`] internally.
    pub fn next_timestamp(&self) -> MavTimestamp {
        self.last_timestamp.next()
    }

    /// ⚠ **DANGER** ⚠
    /// Applies [`SignStrategy`] to a frame.
    ///
    /// This method should never be exposed to a user as it relies on preliminary frame validation.
    fn sign_for_strategy<V: MaybeVersioned>(&self, frame: &mut Frame<V>, strategy: SignStrategy) {
        match strategy {
            SignStrategy::Sign => {
                if !frame.is_signed() {
                    self.sign_frame(frame);
                }
            }
            SignStrategy::ReSign => {
                self.sign_frame(frame);
            }
            SignStrategy::Strip => {
                frame.remove_signature();
            }
            SignStrategy::Strict => {}
            SignStrategy::Proxy => {}
        }
    }
}

impl UniqueMavTimestamp {
    /// Creates a new [`UniqueMavTimestamp`] which is just a moment behind the current time.
    pub fn new() -> Self {
        Self(Arc::new(AtomicU64::new(
            MavTimestamp::from(SystemTime::now()).as_raw_u64() - 1,
        )))
    }

    /// Creates a new [`UniqueMavTimestamp`] from [`MavTimestamp`].
    pub fn init(timestamp: MavTimestamp) -> Self {
        Self(Arc::new(AtomicU64::new(timestamp.as_raw_u64())))
    }

    /// Returns the current timestamp.
    pub fn last(&self) -> MavTimestamp {
        MavTimestamp::from_raw_u64(self.0.load(atomic::Ordering::Acquire))
    }

    /// Returns the next MAVLink timestamp that can be used to sign a frame.
    ///
    /// ⚠ It is not strictly guaranteed that the next timestamp will be unique, if multiple clones
    /// of a signer are used. Nevertheless, this method allows to significantly reduce timestamp
    /// collisions.
    pub fn next(&self) -> MavTimestamp {
        let last_timestamp = self.0.fetch_add(1, atomic::Ordering::Acquire);
        let mut timestamp = MavTimestamp::from(SystemTime::now());

        if timestamp.as_raw_u64() <= last_timestamp {
            timestamp = MavTimestamp::from_raw_u64(last_timestamp + 1);
        } else {
            self.0
                .store(timestamp.as_raw_u64(), atomic::Ordering::Release);
        }

        timestamp
    }
}

impl Default for UniqueMavTimestamp {
    /// Creates a default [`UniqueMavTimestamp`], that is just a moment behind the current time.
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for UniqueMavTimestamp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("UniqueMavTimestamp")
            .field(&self.last())
            .finish()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for UniqueMavTimestamp {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u64(self.last().as_raw_u64())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for UniqueMavTimestamp {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let value = u64::deserialize(d)?;
        Ok(UniqueMavTimestamp::init(MavTimestamp::from_raw_u64(value)))
    }
}

/// Builder for [`MessageSigner`]
pub mod builder {
    use super::*;

    /// Marker for [`MessageSignerBuilder`] which defines whether [`MessageSignerBuilder::key`] was set.
    pub trait MaybeSecretKey: Clone + Debug {}

    /// Marks [`MessageSignerBuilder`] without secret key.
    #[derive(Clone, Debug)]
    pub struct NoSecretKey;
    impl MaybeSecretKey for NoSecretKey {}

    /// Marks [`MessageSignerBuilder`] with secret key being set.
    #[derive(Clone, Debug)]
    pub struct HasSecretKey(SecretKey);
    impl MaybeSecretKey for HasSecretKey {}

    /// Marker for [`MessageSignerBuilder`] which defines whether [`MessageSignerBuilder::link_id`] was set.
    pub trait MaybeLinkId: Copy + Clone + Debug {}

    /// Marks [`MessageSignerBuilder`] without link `ID`.
    #[derive(Copy, Clone, Debug)]
    pub struct NoLinkId;
    impl MaybeLinkId for NoLinkId {}

    /// Marks [`MessageSignerBuilder`] without link `ID` being set.
    #[derive(Copy, Clone, Debug)]
    pub struct HasLinkId(SignedLinkId);
    impl MaybeLinkId for HasLinkId {}

    /// Builder for [`MessageSigner`].
    #[derive(Clone, Debug)]
    pub struct MessageSignerBuilder<L: MaybeLinkId, K: MaybeSecretKey> {
        link_id: L,
        key: K,
        incoming: Option<SignStrategy>,
        outgoing: Option<SignStrategy>,
        links: HashMap<SignedLinkId, SecretKey>,
    }

    impl Default for MessageSignerBuilder<NoLinkId, NoSecretKey> {
        fn default() -> Self {
            Self::new()
        }
    }

    impl<K: MaybeSecretKey> MessageSignerBuilder<NoLinkId, K> {
        /// Set secret key.
        pub fn link_id(self, link_id: SignedLinkId) -> MessageSignerBuilder<HasLinkId, K> {
            MessageSignerBuilder {
                link_id: HasLinkId(link_id),
                key: self.key,
                incoming: self.incoming,
                outgoing: self.outgoing,
                links: self.links,
            }
        }
    }

    impl MessageSignerBuilder<NoLinkId, NoSecretKey> {
        /// Creates a new instance of [`MessageSignerBuilder`].
        ///
        /// # Usage
        ///
        /// ```rust
        /// use maviola::prelude::*;
        ///
        /// MessageSigner::builder()
        ///     .link_id(1)
        ///     .key("main key")
        ///     .incoming(SignStrategy::Sign)
        ///     .outgoing(SignStrategy::Strict)
        ///     .add_link(2, "key for the link #2")
        ///     .add_link(3, "key for the link #3")
        ///     .build();
        /// ```
        pub fn new() -> Self {
            Self {
                link_id: NoLinkId,
                key: NoSecretKey,
                incoming: None,
                outgoing: None,
                links: Default::default(),
            }
        }
    }

    impl<L: MaybeLinkId> MessageSignerBuilder<L, NoSecretKey> {
        /// Set secret key.
        pub fn key<K: Into<SecretKey>>(self, key: K) -> MessageSignerBuilder<L, HasSecretKey> {
            MessageSignerBuilder {
                link_id: self.link_id,
                key: HasSecretKey(key.into()),
                incoming: self.incoming,
                outgoing: self.outgoing,
                links: self.links,
            }
        }
    }

    impl<L: MaybeLinkId, K: MaybeSecretKey> MessageSignerBuilder<L, K> {
        /// Define incoming signing strategy.
        pub fn incoming(self, strategy: SignStrategy) -> MessageSignerBuilder<L, K> {
            Self {
                link_id: self.link_id,
                key: self.key,
                incoming: Some(strategy),
                outgoing: self.outgoing,
                links: self.links,
            }
        }

        /// Define outgoing signing strategy.
        pub fn outgoing(self, strategy: SignStrategy) -> MessageSignerBuilder<L, K> {
            Self {
                link_id: self.link_id,
                key: self.key,
                incoming: self.incoming,
                outgoing: Some(strategy),
                links: self.links,
            }
        }
    }

    impl MessageSignerBuilder<HasLinkId, HasSecretKey> {
        /// Adds additional link.
        ///
        /// ⚠ If `link_id` is the same as the one specified for the main
        /// [`MessageSignerBuilder::link_id`], then the main key will be replaced with the provided one.
        ///
        /// Extra links are used for confirming that processed frames are correctly signed.
        pub fn add_link<K: Into<SecretKey>>(mut self, link_id: SignedLinkId, key: K) -> Self {
            let key = key.into();
            if self.link_id.0 == link_id {
                self.key.0 = key.clone();
            }
            self.links.insert(link_id, key);
            self.link_id = HasLinkId(link_id);
            self
        }
    }

    impl MessageSignerBuilder<HasLinkId, HasSecretKey> {
        /// Builds [`MessageSigner`].
        pub fn build(mut self) -> MessageSigner {
            self.links.insert(self.link_id.0, self.key.0.clone());

            MessageSigner {
                link_id: self.link_id.0,
                incoming: self.incoming.unwrap_or_default(),
                outgoing: self.outgoing.unwrap_or_default(),
                links: self.links,
                last_timestamp: Default::default(),
            }
        }
    }

    impl From<MessageSignerBuilder<HasLinkId, HasSecretKey>> for MessageSigner {
        #[inline]
        fn from(value: MessageSignerBuilder<HasLinkId, HasSecretKey>) -> Self {
            value.build()
        }
    }
}
