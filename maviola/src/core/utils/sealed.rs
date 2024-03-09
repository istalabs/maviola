use crate::protocol::Unset;

pub trait Sealed {}

impl Sealed for Unset {}
