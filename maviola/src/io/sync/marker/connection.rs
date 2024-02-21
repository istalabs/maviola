use mavio::protocol::MaybeVersioned;

use crate::io::marker::{HasConnConf, MaybeConnConf};
use crate::io::sync::connection::ConnectionConf;
use crate::utils::Sealed;

/// Variant of a node configuration which has a synchronous connection config.
pub struct ConnConf<V: MaybeVersioned>(pub(crate) Box<dyn ConnectionConf<V>>);
impl<V: MaybeVersioned> Sealed for ConnConf<V> {}
impl<V: MaybeVersioned> HasConnConf for ConnConf<V> {}
impl<V: MaybeVersioned> MaybeConnConf for ConnConf<V> {}
