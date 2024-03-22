use std::fmt::{Debug, Formatter};

use crate::dialects::Minimal;
use crate::error::FrameError;
use crate::protocol::{DialectSpec, MessageId, MessageInfo};

use crate::prelude::*;

/// Container for multiple MAVLink dialect specifications, that contains one distinct main dialect.
#[derive(Clone)]
pub struct KnownDialects {
    main: &'static str,
    dialects: Vec<&'static DialectSpec>,
    allow_unknown: bool,
}

impl KnownDialects {
    /// Creates a default [`KnownDialects`] instance with [`DefaultDialect`] as a default main
    /// dialect and [`Minimal`] dialect among known dialects.
    pub fn new() -> Self {
        Self {
            main: DefaultDialect::name(),
            dialects: vec![Minimal::spec(), DefaultDialect::spec()],
            allow_unknown: false,
        }
    }

    /// Adds dialect specification as a main dialect.
    ///
    /// **⚠** The [`Minimal`] dialect will always remain in the list of the known dialects no matter
    /// what you are doing. You can't add a custom dialect with the name "minimal".
    pub fn with_dialect(mut self, dialect: &'static DialectSpec) -> Self {
        self.main = dialect.name();
        self = self.with_known_dialect(dialect);
        self
    }

    /// Adds dialect specification as a secondary (known) dialect.
    ///
    /// If for some reason you are using a custom dialect with the same name as one of the known
    /// dialects, then new dialect will override the existing one.
    ///
    /// The [`main`] dialect won't be overridden by this method.
    ///
    /// **⚠** The [`Minimal`] dialect will always remain in the list of the known dialects no matter
    /// what you are doing.
    ///
    /// [`main`]: Self::main
    pub fn with_known_dialect(mut self, dialect: &'static DialectSpec) -> Self {
        if !self.contains(dialect.name()) {
            self.dialects.push(dialect);
        } else if dialect.name() != Minimal::name() && dialect.name() != self.main {
            let mut dialects = Vec::new();
            for _dialect in self.dialects {
                if _dialect.name() != dialect.name() {
                    dialects.push(_dialect);
                }
                dialects.push(dialect);
            }
            self.dialects = dialects;
        }
        self
    }

    /// Removes [`DefaultDialect`] dialect from the list of known dialects.
    ///
    /// If default dialect is currently the [`main`] one, then the main dialect will be set to
    /// [`Minimal`].
    ///
    /// **⚠** The [`Minimal`] dialect will always remain in the list of the known dialects no matter
    /// what you are doing.
    ///
    /// [`main`]: Self::main
    pub fn without_default_dialect(mut self) -> Self {
        if DefaultDialect::name() == Minimal::name() {
            return self;
        }

        self.dialects = self
            .dialects
            .iter()
            .filter(|&&dialect| dialect.name() != DefaultDialect::name())
            .copied()
            .collect();

        if DefaultDialect::name() == self.main {
            self.main = Minimal::name();
        }

        self
    }

    /// Allow unknown dialects (default is `false`).
    pub fn with_allow_unknown(mut self, value: bool) -> Self {
        self.allow_unknown = value;
        self
    }

    /// Main dialect specification.
    pub fn main(&self) -> &'static DialectSpec {
        self.get(self.main).unwrap()
    }

    /// Supported dialect specifications.
    pub fn known(&self) -> impl Iterator<Item = &DialectSpec> {
        self.dialects.clone().into_iter()
    }

    /// Returns `true`, if unknown dialects are allowed (default is `false`).
    #[inline(always)]
    pub fn allow_unknown(&self) -> bool {
        self.allow_unknown
    }

    /// Returns `true`, if dialect specification with provided `name` is among the known dialects.
    pub fn contains(&self, name: &str) -> bool {
        for &dialect in &self.dialects {
            if dialect.name() == name {
                return true;
            }
        }
        false
    }

    /// Returns dialect specification by dialect name.
    pub fn get(&self, name: &str) -> Option<&'static DialectSpec> {
        self.dialects
            .iter()
            .find(|&&dialect| dialect.name() == name)
            .copied()
    }

    /// Checks if message `id` belongs to the main dialect.
    #[inline(always)]
    pub fn contains_message_id(&self, id: MessageId) -> bool {
        self.main().message_info(id).is_ok()
    }

    /// Checks, that provided message `id` belongs to a known dialect.
    pub fn is_known_message_id(&self, id: MessageId) -> bool {
        for &dialect in &self.dialects {
            if dialect.message_info(id).is_ok() {
                return true;
            }
        }
        false
    }

    /// Returns first dialect, that contains provided message `id`.
    ///
    /// The [`KnownDialects::main`] dialect will always be checked first.
    pub fn get_first_by_message_id(&self, id: MessageId) -> Option<&'static DialectSpec> {
        if self.contains_message_id(id) {
            return Some(self.main());
        }
        self.dialects
            .iter()
            .find(|&&dialect| dialect.name() != self.main && dialect.message_info(id).is_ok())
            .copied()
    }

    /// Get MAVLink message info by specified message `id` from the known dialects.
    ///
    /// The main dialect will always be checked first.
    pub fn message_info_by_id(&self, id: MessageId) -> Option<&'static MessageInfo> {
        if let Ok(info) = self.main().message_info(id) {
            return Some(info);
        }

        for dialect in &self.dialects {
            if self.main == dialect.name() {
                continue;
            }
            if let Ok(info) = dialect.message_info(id) {
                return Some(info);
            }
        }

        None
    }

    /// Validates provided MAVLink frame.
    ///
    /// If frame does not belong to any known dialect and [`KnownDialects::allow_unknown`] is
    /// `false`, then [`FrameError::NotInDialect`] will be returned.
    pub fn validate_frame<V: MaybeVersioned>(
        &self,
        frame: &Frame<V>,
    ) -> core::result::Result<(), FrameError> {
        if !self.allow_unknown && !self.is_known_message_id(frame.message_id()) {
            return Err(FrameError::NotInDialect(frame.message_id()));
        }
        Ok(())
    }

    pub(in crate::protocol) fn as_slice(&self) -> &[&DialectSpec] {
        self.dialects.as_slice()
    }
}

impl Default for KnownDialects {
    fn default() -> Self {
        KnownDialects::new()
    }
}

impl Debug for KnownDialects {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut result = f.debug_tuple("KnownDialects");
        for dialect in &self.dialects {
            result.field(&dialect.name());
        }
        result.finish()
    }
}
