//! Common utils.

mod sealed;
#[cfg(test)]
#[allow(dead_code)]
pub(crate) mod test;
mod unique_id;

pub(crate) use sealed::Sealed;
pub(crate) use unique_id::UniqueId;
