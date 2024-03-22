#![allow(non_snake_case)]

/*!
# 📖 Maviola Playbook

Maviola is a high-level [MAVLink](https://mavlink.io/en/) communication library written in Rust.
It provides abstractions such as communication nodes and implements **stateful** features of MAVLink
protocol: sequencing, message signing, automatic heartbeats, and so on.

This library is a part of [Mavka](https://mavka.gitlab.io/home/) toolchain. It is based on
[Mavio](https://gitlab.com/mavka/libs/mavio), a low-level MAVLink library, and compatible with
[MAVSpec](https://gitlab.com/mavka/libs/mavspec) MAVLink dialects generator.

This documentation provides in-depth explanation of available features. We suggest to begin from
the [Quickstart](crate::docs::a1__quickstart) and then move to other sections.

If you are interested in the reasoning behind this library, we have a corresponding
[Why Maviola?](crate::docs::a2__overview#why-maviola).

## Contents

1. Basics
    1. [Quickstart](crate::docs::a1__quickstart)
    1. [Overview](crate::docs::a2__overview)
    1. [Synchronous API](crate::docs::a3__sync_api)
    1. [Asynchronous API](crate::docs::a4__async_api)
1. Advanced Usage
    1. [Dialect Constraints](crate::docs::b1__dialect_constraints)
    1. [Message Signing](crate::docs::b2__signing)
    1. [Compatibility Checks](crate::docs::b3__compat_checks)
    1. [Networks & Routing](crate::docs::b4__networks_and_routing)
1. Customization
    1. [Custom Dialects](crate::docs::c1__custom_dialects)
    1. [Custom Transport](crate::docs::c2__custom_transport)
    1. [Custom Processing](crate::docs::c3__custom_processing)
    1. [Ad-hoc Dialects](crate::docs::c4__ad_hoc_dialects)

<em>[Quickstart →](crate::docs::a1__quickstart)</em>
*/

pub mod a1__quickstart;
pub mod a2__overview;
pub mod a3__sync_api;
pub mod a4__async_api;
pub mod b1__dialect_constraints;
pub mod b2__signing;
pub mod b3__compat_checks;
pub mod b4__networks_and_routing;
pub mod c1__custom_dialects;
pub mod c2__custom_transport;
pub mod c3__custom_processing;
pub mod c4__ad_hoc_dialects;
