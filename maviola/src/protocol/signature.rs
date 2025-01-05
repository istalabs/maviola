//! MAVLink [message signing](https://mavlink.io/en/guide/message_signing.html) tools.

use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::sync::atomic::AtomicU64;
use std::sync::{atomic, Arc};
use std::time::SystemTime;

use crate::error::SignatureError;
use crate::protocol::{
    MavSha256, MavTimestamp, MessageId, SecretKey, Sign, SignedLinkId, Signer, SigningConf,
};

use crate::prelude::*;

pub use builder::FrameSignerBuilder;
use builder::{NoLinkId, NoSecretKey};

/// <sup>[`serde`](https://serde.rs) | [`specta`](https://crates.io/crates/specta)</sup>
/// MAVLink [message signing](https://mavlink.io/en/guide/message_signing.html) manager.
///
/// [`FrameSigner`] allows to verify and sign outgoing and incoming frames. There are several
/// signing strategies defined by [`SignStrategy`] which can be specified both for
/// [`FrameSigner::incoming`] and [`FrameSigner::outgoing`] frames.
///
/// Each instance of a manager is configured with the main [`FrameSigner::link_id`] and the main
/// [`FrameSigner::key`]. These will be used to sign frames.
///
/// It is possible to add additional links with corresponding secret keys. These links will be used
/// to validate already signed frames. Depending on a [`SignStrategy`], these frames can be either
/// rejected, kept as they are, or re-signed with the main key and link `ID`. All supported links
/// (including the main one) can be accessed with [`FrameSigner::links`].
///
/// **⚠** Secret keys are excluded from [Serde](https://serde.rs) serialization.
///
/// # Examples
///
/// ```rust
/// use maviola::prelude::*;
///
/// let signer = FrameSigner::builder()
///     .link_id(1)                         // Set main link `ID`
///     .key("main key")                    // Set main key
///     .incoming(SignStrategy::ReSign)     // All incoming frames will be re-signed
///     .outgoing(SignStrategy::Strict)     // Outgoing frames will be signed
///     .add_link(2, "key for the link #2") // Add extra link
///     .build();
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FrameSigner {
    link_id: SignedLinkId,
    incoming: SignStrategy,
    outgoing: SignStrategy,
    unknown_links: SignStrategy,
    #[cfg_attr(feature = "serde", serde(skip_serializing))]
    links: HashMap<SignedLinkId, SecretKey>,
    last_timestamp: UniqueMavTimestamp,
    exclude: HashSet<MessageId>,
}

/// <sup>[`serde`](https://serde.rs) | [`specta`](https://crates.io/crates/specta)</sup>
/// Message signing strategy.
///
/// Defines how message signing will be applied.
///
/// By default, the [`SignStrategy::Sign`] strategy will be applied.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SignStrategy {
    /// Apply message signing for all messages. Sign unsigned messages.
    ///
    /// Unsigned messages will be signed. Messages with incorrect signature will be rejected.
    ///
    /// For messages with unknown links the main [`link_id`] and [`key`] will be used for
    /// validation.
    ///
    /// When set as a strategy for [`unknown_links`], then valid messages with unknown links will
    /// keep their signature for [`ReSign`] outgoing / incoming strategies.
    ///
    /// [`Versionless`] frames of `MAVLink 1` protocol will be passed without change.
    ///
    /// [`link_id`]: FrameSigner::link_id
    /// [`key`]: FrameSigner::key
    /// [`unknown_links`]: FrameSigner::unknown_links
    /// [`ReSign`]: SignStrategy::ReSign
    #[default]
    Sign,
    /// Enforce message signing for all messages. Re-sign messages with correct signing using the
    /// main [`link_id`] and [`key`].
    ///
    /// Unsigned messages will be signed. Messages with incorrect signature will be rejected.
    ///
    /// For messages with unknown links the main [`link_id`] and [`key`] will be used for
    /// validation.
    ///
    /// When set as a strategy for [`unknown_links`], then valid messages with unknown links will
    /// be re-signed for both [`Sign`] and [`ReSign`] outgoing / incoming strategies using the main
    /// [`link_id`] and [`key`].
    ///
    /// [`Versionless`] frames of `MAVLink 1` protocol will be passed without change.
    ///
    /// [`link_id`]: FrameSigner::link_id
    /// [`key`]: FrameSigner::key
    /// [`unknown_links`]: FrameSigner::unknown_links
    /// [`Sign`]: SignStrategy::Sign
    /// [`ReSign`]: SignStrategy::ReSign
    ReSign,
    /// Apply message signing for all messages. Reject messages without signing or with incorrect
    /// signatures.
    ///
    /// Unsigned messages will be rejected. Messages with incorrect signature will be rejected.
    ///
    /// Messages with unknown links will be rejected.
    ///
    /// When set as a strategy for [`unknown_links`], then messages with unknown links
    /// (even when valid) will be considered invalid for [`Sign`] and [`ReSign`] incoming / outgoing
    /// strategies.
    ///
    /// [`Versionless`] frames of `MAVLink 1` protocol will be rejected.
    ///
    /// [`unknown_links`]: FrameSigner::unknown_links
    /// [`Sign`]: SignStrategy::Sign
    /// [`ReSign`]: SignStrategy::ReSign
    Strict,
    /// Pass messages as they are.
    Proxy,
    /// Strip any signing information from messages.
    Strip,
}

/// A trait for entities, that can be converted to [`FrameSigner`].
///
/// Currently, this trait is implemented for [`FrameSigner`] and [`FrameSignerBuilder`].
pub trait IntoFrameSigner {
    /// Convert to a [`FrameSigner`].
    fn into_message_signer(self) -> FrameSigner;
}

/// <sup>[`serde`](https://serde.rs) | [`specta`](https://crates.io/crates/specta)</sup>
/// MAVLink timestamp wrapper that preserve uniqueness of timestamp sequence.
///
/// ⚠ It is not strictly guaranteed that the next timestamp will be unique, if multiple clones
/// of a signer are used. Nevertheless, [`UniqueMavTimestamp`] allows to significantly reduc
/// timestamp collisions.
///
/// Used internally by [`FrameSigner`].
#[derive(Clone)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct UniqueMavTimestamp(Arc<AtomicU64>);

impl FrameSigner {
    /// Creates a [`FrameSigner`] with the main `link_id` / `key` and default strategies.
    ///
    /// # Usage
    ///
    /// ```rust
    /// use maviola::prelude::*;
    ///
    /// let signer = FrameSigner::new(17, "main secret key");
    /// ```
    ///
    /// This is equal to:
    ///
    /// ```rust
    /// use maviola::prelude::*;
    ///
    /// let signer = FrameSigner::builder()
    ///     .link_id(17)
    ///     .key("main secret key")
    ///     .build();
    /// ```
    pub fn new<K: Into<SecretKey>>(link_id: SignedLinkId, key: K) -> Self {
        Self::builder().link_id(link_id).key(key.into()).build()
    }

    /// Instantiates an empty [`FrameSignerBuilder`].
    pub fn builder() -> FrameSignerBuilder<NoLinkId, NoSecretKey> {
        FrameSignerBuilder::new()
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
    ///
    /// The default value is [`SignStrategy::Sign`].
    pub fn incoming(&self) -> SignStrategy {
        self.incoming
    }

    /// Signing strategy for outgoing messages.
    ///
    /// The default value is [`SignStrategy::Sign`].
    pub fn outgoing(&self) -> SignStrategy {
        self.outgoing
    }

    /// Signing strategy for messages with unknown [`link_id`](Self::link_id).
    ///
    /// The default value is [`SignStrategy::Strict`]. Which means that frames with unknown links
    /// are considered to be invalid.
    pub fn unknown_links(&self) -> SignStrategy {
        self.unknown_links
    }

    /// Iterator over supported links.
    ///
    /// Links will always contain the main link `ID` and the main secret key.
    pub fn links(&self) -> impl Iterator<Item = (SignedLinkId, &SecretKey)> {
        self.links.iter().map(|(&link_id, key)| (link_id, key))
    }

    /// Message `IDs` excluded from message signing and verification.
    pub fn exclude(&self) -> impl Iterator<Item = MessageId> {
        self.exclude.clone().into_iter()
    }

    /// Takes incoming frame and processes it according to a [`Self::incoming`] signing strategy.
    #[inline(always)]
    pub fn process_incoming<V: MaybeVersioned>(
        &self,
        frame: &mut Frame<V>,
    ) -> core::result::Result<(), SignatureError> {
        self.process_for_strategy(frame, self.incoming)
    }

    /// Takes outgoing frame and processes it according to a [`Self::outgoing`] signing strategy.
    #[inline(always)]
    pub fn process_outgoing<V: MaybeVersioned>(
        &self,
        frame: &mut Frame<V>,
    ) -> core::result::Result<(), SignatureError> {
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
    ///
    /// If [`Frame::message_id`] in [`FrameSigner::exclude`], then it will be skipped from
    /// validation.
    pub fn process_for_strategy<V: MaybeVersioned>(
        &self,
        frame: &mut Frame<V>,
        strategy: SignStrategy,
    ) -> core::result::Result<(), SignatureError> {
        if self.exclude.contains(&frame.message_id()) {
            return Ok(());
        }
        self.validate_for_strategy(frame, strategy)?;
        self.sign_for_strategy(frame, strategy);
        Ok(())
    }

    /// Validates a [`Frame`] given the provided [`SignStrategy`].
    pub fn validate_for_strategy<V: MaybeVersioned>(
        &self,
        frame: &Frame<V>,
        strategy: SignStrategy,
    ) -> core::result::Result<(), SignatureError> {
        if let SignStrategy::Proxy = strategy {
            return Ok(());
        }

        if let SignStrategy::Strict = strategy {
            if !frame.is_signed() {
                return Err(SignatureError);
            }
        }

        match strategy {
            SignStrategy::Sign | SignStrategy::ReSign | SignStrategy::Strict => {
                if frame.is_signed() && !self.has_valid_signature(frame) {
                    return Err(SignatureError);
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
    /// [`Frame::link_id`]. If such link `ID` is not among the available [`FrameSigner::links`],
    /// then frame will be considered invalid.
    ///
    /// When frame has unknown link `ID`, then the main [`FrameSigner::key`] will be used for
    /// validation. If [`FrameSigner::unknown_links`] is [`SignStrategy::Strict`], then frames
    /// with unknown links will be rejected no matter what.
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
            signer.validate(frame, signature, key)
        } else {
            match self.unknown_links {
                SignStrategy::Sign | SignStrategy::ReSign => {
                    let mut _signer = self.signer();
                    let mut signer = Signer::new(&mut _signer);
                    signer.validate(frame, signature, self.key())
                }
                SignStrategy::Strict => false,
                SignStrategy::Proxy | SignStrategy::Strip => true,
            }
        }
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

    /// <sup>⛔</sup>
    /// ⚠ **DANGER** ⚠ Applies [`SignStrategy`] to a frame.
    ///
    /// This method should be never exposed to a user as it relies on preliminary frame validation.
    fn sign_for_strategy<V: MaybeVersioned>(&self, frame: &mut Frame<V>, strategy: SignStrategy) {
        match strategy {
            SignStrategy::Sign => {
                if self.should_sign(frame) {
                    self.sign_frame(frame);
                }
            }
            SignStrategy::ReSign => {
                if self.should_re_sign(frame) {
                    self.sign_frame(frame);
                }
            }
            SignStrategy::Strip => {
                frame.remove_signature();
            }
            SignStrategy::Strict => {}
            SignStrategy::Proxy => {}
        }
    }

    /// <sup>⛔</sup>
    /// Checks, that frame should be signed for [`SignStrategy::Sign`].
    fn should_sign<V: MaybeVersioned>(&self, frame: &Frame<V>) -> bool {
        if let Some(signature) = frame.signature() {
            self.links.get(&signature.link_id).is_none()
                && (self.unknown_links == SignStrategy::Sign
                    || self.unknown_links == SignStrategy::ReSign)
        } else {
            true
        }
    }

    /// <sup>⛔</sup>
    /// Checks, that frame should be signed for [`SignStrategy::ReSign`].
    fn should_re_sign<V: MaybeVersioned>(&self, frame: &Frame<V>) -> bool {
        if let Some(signature) = frame.signature() {
            if self.links.get(&signature.link_id).is_none() {
                self.unknown_links == SignStrategy::ReSign
            } else {
                true
            }
        } else {
            true
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

impl IntoFrameSigner for FrameSigner {
    /// Passes [`FrameSigner`] without change.
    fn into_message_signer(self) -> FrameSigner {
        self
    }
}

/// Builder for [`FrameSigner`]
pub mod builder {
    use super::*;

    /// Marker for [`FrameSignerBuilder`] which defines whether [`FrameSignerBuilder::key`] was set.
    pub trait MaybeSecretKey: Clone + Debug {}

    /// Marks [`FrameSignerBuilder`] without secret key.
    #[derive(Clone, Debug)]
    pub struct NoSecretKey;
    impl MaybeSecretKey for NoSecretKey {}

    /// Marks [`FrameSignerBuilder`] with secret key being set.
    #[derive(Clone, Debug)]
    pub struct HasSecretKey(SecretKey);
    impl MaybeSecretKey for HasSecretKey {}

    /// Marker for [`FrameSignerBuilder`] which defines whether [`FrameSignerBuilder::link_id`] was set.
    pub trait MaybeLinkId: Copy + Clone + Debug {}

    /// Marks [`FrameSignerBuilder`] without link `ID`.
    #[derive(Copy, Clone, Debug)]
    pub struct NoLinkId;
    impl MaybeLinkId for NoLinkId {}

    /// Marks [`FrameSignerBuilder`] without link `ID` being set.
    #[derive(Copy, Clone, Debug)]
    pub struct HasLinkId(SignedLinkId);
    impl MaybeLinkId for HasLinkId {}

    /// Builder for [`FrameSigner`].
    #[derive(Clone, Debug)]
    pub struct FrameSignerBuilder<L: MaybeLinkId, K: MaybeSecretKey> {
        link_id: L,
        key: K,
        incoming: Option<SignStrategy>,
        outgoing: Option<SignStrategy>,
        unknown_links: Option<SignStrategy>,
        links: HashMap<SignedLinkId, SecretKey>,
        exclude: HashSet<MessageId>,
    }

    impl FrameSignerBuilder<NoLinkId, NoSecretKey> {
        /// Creates a new instance of [`FrameSignerBuilder`].
        ///
        /// # Usage
        ///
        /// ```rust
        /// use maviola::prelude::*;
        ///
        /// FrameSigner::builder()
        ///     .link_id(1)
        ///     .key("main key")
        ///     .incoming(SignStrategy::Sign)
        ///     .outgoing(SignStrategy::Strict)
        ///     .add_link(2, "key for the link #2")
        ///     .add_link(3, "key for the link #3")
        ///     .exclude(&[11, 17, 42])
        ///     .build();
        /// ```
        pub fn new() -> Self {
            Self {
                link_id: NoLinkId,
                key: NoSecretKey,
                incoming: None,
                outgoing: None,
                unknown_links: None,
                links: Default::default(),
                exclude: Default::default(),
            }
        }
    }

    impl<K: MaybeSecretKey> FrameSignerBuilder<NoLinkId, K> {
        /// Set [`FrameSigner::link_id`].
        pub fn link_id(self, link_id: SignedLinkId) -> FrameSignerBuilder<HasLinkId, K> {
            FrameSignerBuilder {
                link_id: HasLinkId(link_id),
                key: self.key,
                incoming: self.incoming,
                outgoing: self.outgoing,
                unknown_links: self.unknown_links,
                links: self.links,
                exclude: self.exclude,
            }
        }
    }

    impl<L: MaybeLinkId> FrameSignerBuilder<L, NoSecretKey> {
        /// Set [`FrameSigner::key`].
        pub fn key<K: Into<SecretKey>>(self, key: K) -> FrameSignerBuilder<L, HasSecretKey> {
            FrameSignerBuilder {
                link_id: self.link_id,
                key: HasSecretKey(key.into()),
                incoming: self.incoming,
                outgoing: self.outgoing,
                unknown_links: self.unknown_links,
                links: self.links,
                exclude: self.exclude,
            }
        }
    }

    impl<L: MaybeLinkId, K: MaybeSecretKey> FrameSignerBuilder<L, K> {
        /// Set [`FrameSigner::incoming`].
        pub fn incoming(self, strategy: SignStrategy) -> Self {
            Self {
                incoming: Some(strategy),
                ..self
            }
        }

        /// Set [`FrameSigner::outgoing`].
        pub fn outgoing(self, strategy: SignStrategy) -> Self {
            Self {
                outgoing: Some(strategy),
                ..self
            }
        }

        /// <sup>`⍚` |</sup>
        /// Set [`FrameSigner::unknown_links`].
        ///
        /// Default value is [`SignStrategy::Strict`].
        ///
        /// Available only when `unstable` Cargo feature is set.
        #[cfg(feature = "unstable")]
        pub fn unknown_links(self, strategy: SignStrategy) -> Self {
            Self {
                unknown_links: Some(strategy),
                ..self
            }
        }

        /// Set [`FrameSigner::exclude`].
        pub fn exclude(self, message_ids: &[MessageId]) -> Self {
            Self {
                exclude: HashSet::from_iter(message_ids.iter().copied()),
                ..self
            }
        }
    }

    impl FrameSignerBuilder<HasLinkId, HasSecretKey> {
        /// Adds additional link to [`FrameSigner::links`].
        ///
        /// ⚠ If `link_id` is the same as the one specified for the main
        /// [`FrameSignerBuilder::link_id`], then the main key will be replaced with the provided one.
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

    impl FrameSignerBuilder<HasLinkId, HasSecretKey> {
        /// Builds [`FrameSigner`].
        pub fn build(mut self) -> FrameSigner {
            self.links.insert(self.link_id.0, self.key.0.clone());

            FrameSigner {
                link_id: self.link_id.0,
                incoming: self.incoming.unwrap_or_default(),
                outgoing: self.outgoing.unwrap_or_default(),
                unknown_links: self.unknown_links.unwrap_or(SignStrategy::Strict),
                links: self.links,
                last_timestamp: Default::default(),
                exclude: self.exclude,
            }
        }
    }

    impl Default for FrameSignerBuilder<NoLinkId, NoSecretKey> {
        fn default() -> Self {
            Self::new()
        }
    }

    impl From<FrameSignerBuilder<HasLinkId, HasSecretKey>> for FrameSigner {
        #[inline]
        fn from(value: FrameSignerBuilder<HasLinkId, HasSecretKey>) -> Self {
            value.build()
        }
    }

    impl IntoFrameSigner for FrameSignerBuilder<HasLinkId, HasSecretKey> {
        /// Builds [`FrameSigner`] from [`FrameSignerBuilder`].
        fn into_message_signer(self) -> FrameSigner {
            self.build()
        }
    }
}
