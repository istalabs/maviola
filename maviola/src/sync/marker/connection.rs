use crate::protocol::MaybeVersioned;

use crate::core::marker::{HasConnConf, MaybeConnConf};
use crate::core::utils::Sealed;
use crate::sync::conn::ConnectionBuilder;

/// <sup>[`sync`](crate::sync)</sup>
/// Variant of a node configuration which has a synchronous connection config.
#[derive(Debug)]
pub struct ConnConf<V: MaybeVersioned>(pub(crate) Box<dyn ConnectionBuilder<V>>);
impl<V: MaybeVersioned> Sealed for ConnConf<V> {}
impl<V: MaybeVersioned> HasConnConf for ConnConf<V> {}
impl<V: MaybeVersioned> MaybeConnConf for ConnConf<V> {}
