use maviola::protocol::{FrameSigner, SignStrategy};

#[test]
fn define_signing_config() {
    FrameSigner::builder()
        .key("abcdef")
        .link_id(1)
        .incoming(SignStrategy::Sign)
        .outgoing(SignStrategy::Strict)
        .build();
}
