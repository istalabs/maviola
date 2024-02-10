use maviola::io::signature::{SignConf, SignStrategy};

#[test]
fn define_signing_config() {
    SignConf::builder()
        .key("abcdef")
        .incoming(SignStrategy::Proxy)
        .outgoing(SignStrategy::Reject)
        .build();
}
