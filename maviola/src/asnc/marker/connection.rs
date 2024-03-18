use crate::asnc::io::ConnectionBuilder;
use crate::core::marker::{HasConnConf, MaybeConnConf};
use crate::core::utils::Sealed;

use crate::prelude::*;

/// <sup>[`async`](crate::asnc)</sup>
/// Variant of a node configuration which has an asynchronous connection config.
#[derive(Debug)]
pub struct AsyncConnConf<V: MaybeVersioned>(pub(crate) Box<dyn ConnectionBuilder<V>>);

unsafe impl<V: MaybeVersioned> Sync for AsyncConnConf<V> {}

impl<V: MaybeVersioned> Sealed for AsyncConnConf<V> {}
impl<V: MaybeVersioned + 'static> HasConnConf for AsyncConnConf<V> {
    fn is_repairable(&self) -> bool {
        self.0.is_repairable()
    }
}
impl<V: MaybeVersioned> MaybeConnConf for AsyncConnConf<V> {}

impl<V: MaybeVersioned + 'static> AsyncConnConf<V> {
    pub(in crate::asnc) fn new(builder: impl ConnectionBuilder<V> + 'static) -> Self {
        Self(Box::new(builder))
    }

    pub(in crate::asnc) fn connection(&self) -> &dyn ConnectionBuilder<V> {
        self.0.as_ref()
    }
}

impl<V: MaybeVersioned + 'static> Clone for AsyncConnConf<V> {
    fn clone(&self) -> Self {
        self.0.to_conf()
    }
}
