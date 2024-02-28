use crate::protocol::MaybeVersioned;

use crate::asnc::io::ConnectionBuilder;
use crate::core::marker::{HasConnConf, MaybeConnConf};
use crate::core::utils::Sealed;

/// <sup>[`async`](crate::asnc)</sup>
/// Variant of a node configuration which has an asynchronous connection config.
#[derive(Debug)]
pub struct AsyncConnConf<V: MaybeVersioned>(pub(crate) Box<dyn ConnectionBuilder<V>>);
impl<V: MaybeVersioned> Sealed for AsyncConnConf<V> {}
impl<V: MaybeVersioned> HasConnConf for AsyncConnConf<V> {}
impl<V: MaybeVersioned> MaybeConnConf for AsyncConnConf<V> {}
