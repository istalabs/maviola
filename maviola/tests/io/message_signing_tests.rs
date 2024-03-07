use maviola::protocol::{MessageSigner, SignStrategy};

#[test]
fn define_signing_config() {
    MessageSigner::builder()
        .key("abcdef")
        .link_id(1)
        .incoming(SignStrategy::Sign)
        .outgoing(SignStrategy::Strict)
        .build();
}
