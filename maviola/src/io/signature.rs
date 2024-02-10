//! MAVLink [message signing](https://mavlink.io/en/guide/message_signing.html) tools.

use mavio::protocol::SecretKey;

use builder::{HasSecretKey, NoSecretKey, SignConfBuilder};

/// MAVLink message signing configuration.
#[derive(Clone, Debug)]
pub struct SignConf {
    key: SecretKey,
    incoming: SignStrategy,
    outgoing: SignStrategy,
}

/// Message signing strategy.
///
/// Defines how message signing will be applied.
///
/// By default, the most strict [`SignStrategy::Reject`] strategy will be applied.
#[derive(Clone, Copy, Debug, Default)]
pub enum SignStrategy {
    /// Apply message signing for all messages. Reject messages with incorrect signing.
    ///
    /// Unsigned messages will be signed. Messages with incorrect signature will be rejected.
    #[default]
    Reject,
    /// Apply message signing for all messages. Sign unsigned messages.
    ///
    /// Unsigned messages will be signed. Messages with incorrect signature will be passed through.
    Proxy,
    /// Enforce message signing for all messages. Re-sign messages with incorrect signing.
    ///
    /// Unsigned messages will be signed. Messages with incorrect signature will be re-signed.
    Resign,
    /// Strip any signing information from messages.
    Strip,
    /// Pass messages as they are.
    Ignore,
}

impl From<SignConfBuilder<HasSecretKey>> for SignConf {
    #[inline]
    fn from(value: SignConfBuilder<HasSecretKey>) -> Self {
        value.build()
    }
}

impl SignConf {
    /// Instantiates an empty [`SignConfBuilder`].
    pub fn builder() -> SignConfBuilder<NoSecretKey> {
        SignConfBuilder::new()
    }

    /// Secret key.
    pub fn key(&self) -> &SecretKey {
        &self.key
    }

    /// Signing strategy for incoming messages.
    pub fn incoming(&self) -> SignStrategy {
        self.incoming
    }

    /// Signing strategy for outgoing messages.
    pub fn outgoing(&self) -> SignStrategy {
        self.outgoing
    }
}

/// Builder for [`SignConf`]
pub mod builder {
    use super::*;

    /// Marker for [`SignConfBuilder`] which defines whether [`SignConfBuilder::key`] was set.
    pub trait IsSecretKeySet {}

    /// Marks [`SignConfBuilder`] without secret key.
    #[derive(Clone, Debug)]
    pub struct NoSecretKey();
    impl IsSecretKeySet for NoSecretKey {}

    /// Marks [`SignConfBuilder`] with secret key being set.
    #[derive(Clone, Debug)]
    pub struct HasSecretKey(SecretKey);
    impl IsSecretKeySet for HasSecretKey {}

    /// Builder for [`SignConf`].
    #[derive(Clone, Debug)]
    pub struct SignConfBuilder<S: IsSecretKeySet> {
        key: S,
        incoming: Option<SignStrategy>,
        outgoing: Option<SignStrategy>,
    }

    impl Default for SignConfBuilder<NoSecretKey> {
        fn default() -> Self {
            Self::new()
        }
    }

    impl SignConfBuilder<NoSecretKey> {
        /// Creates a new instance of [`SignConfBuilder`].
        ///
        /// # Usage
        ///
        /// ```rust
        /// use maviola::io::signature::{SignConf, SignStrategy};
        ///
        /// SignConf::builder()
        ///     .key("abcdef")
        ///     .incoming(SignStrategy::Proxy)
        ///     .outgoing(SignStrategy::Reject)
        ///     .build();
        /// ```
        pub fn new() -> Self {
            Self {
                key: NoSecretKey(),
                incoming: None,
                outgoing: None,
            }
        }

        /// Set secret key.
        pub fn key<K: Into<SecretKey>>(self, key: K) -> SignConfBuilder<HasSecretKey> {
            SignConfBuilder {
                key: HasSecretKey(key.into()),
                incoming: self.incoming,
                outgoing: self.outgoing,
            }
        }
    }

    impl SignConfBuilder<HasSecretKey> {
        /// Builds [`SignConf`].
        pub fn build(self) -> SignConf {
            SignConf {
                key: self.key.0,
                incoming: self.incoming.unwrap_or_default(),
                outgoing: self.outgoing.unwrap_or_default(),
            }
        }
    }

    impl<K: IsSecretKeySet> SignConfBuilder<K> {
        /// Define incoming signing strategy.
        pub fn incoming(self, strategy: SignStrategy) -> SignConfBuilder<K> {
            Self {
                key: self.key,
                incoming: Some(strategy),
                outgoing: self.outgoing,
            }
        }

        /// Define outgoing signing strategy.
        pub fn outgoing(self, strategy: SignStrategy) -> SignConfBuilder<K> {
            Self {
                key: self.key,
                incoming: self.incoming,
                outgoing: Some(strategy),
            }
        }
    }
}
