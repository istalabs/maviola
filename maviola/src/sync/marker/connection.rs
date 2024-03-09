use crate::core::marker::{HasConnConf, MaybeConnConf};
use crate::core::utils::Sealed;
use crate::sync::io::ConnectionBuilder;

use crate::prelude::*;

/// <sup>[`sync`](crate::sync)</sup>
/// Variant of a node configuration which has a synchronous connection config.
#[derive(Debug)]
pub struct ConnConf<V: MaybeVersioned>(pub(in crate::sync) Box<dyn ConnectionBuilder<V>>);

impl<V: MaybeVersioned> Sealed for ConnConf<V> {}
impl<V: MaybeVersioned> HasConnConf for ConnConf<V> {
    fn is_repairable(&self) -> bool {
        self.0.is_repairable()
    }
}
impl<V: MaybeVersioned> MaybeConnConf for ConnConf<V> {}

impl<V: MaybeVersioned> ConnConf<V> {
    pub(in crate::sync) fn new(builder: impl ConnectionBuilder<V> + 'static) -> Self {
        Self(Box::new(builder))
    }

    pub(in crate::sync) fn connection(&self) -> &dyn ConnectionBuilder<V> {
        self.0.as_ref()
    }
}

impl<V: MaybeVersioned> Clone for ConnConf<V> {
    fn clone(&self) -> Self {
        self.0.to_conf()
    }
}
