/*!
# 📖 2.3. Compatibility Checks

<em>[← Message Signing](crate::docs::b2__signing) | [Networks & Routing →](crate::docs::b4__networks_and_routing)</em>

Maviola provides an API for automatic checking and setting compatibility and incompatibility flags
for MAVLink frames.

To define compatibility / incompatibility settings for a [`Node`], we need to use a `compat` method
of the node builder.

```rust,no_run
# use maviola::prelude::*;
# use maviola::sync::prelude::*;
use maviola::protocol::{CompatFlags, IncompatFlags};

let node = Node::sync::<V2>()
    .compat(CompatProcessor::builder()
        .incompat_flags(IncompatFlags::BIT_2 | IncompatFlags::BIT_5)
        .compat_flags(CompatFlags::BIT_3 | CompatFlags::BIT_4)
        .outgoing(CompatStrategy::Enforce)
        .incoming(CompatStrategy::Reject)
    )
    # .connection(TcpClient::new("127.0.0.1:5600").unwrap())
    /* other node settings */
    .build().unwrap();
```

Check [`CompatProcessor`] documentation for details.

<em>[← Message Signing](crate::docs::b2__signing) | [Networks & Routing →](crate::docs::b4__networks_and_routing)</em>
 */

#[cfg(doc)]
use crate::prelude::*;
#[cfg(doc)]
use crate::protocol::*;
