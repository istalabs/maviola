/*!
# 📖 4.1. Testing

<em>[← Implementation Notes](crate::docs::e2__implementation)</em>

Since we have two types of API and several feature flags, the proper testing could be cumbersome.
Here is the list of commands required to ensure, that your pull request will pass the
[CI](https://gitlab.com/mavka/libs/maviola/-/pipelines) checks related to testing.

Common tests (the last one and is not essential for fast checks):

```shell
cargo test --no-default-features --lib --tests --bins
cargo test --features sync,async,unstable,unsafe --lib --tests --bins
cargo test --all-features --lib --tests --bins
```

Documentation tests (again, the last one can be skipped):

```shell
cargo test --no-default-features --features test_utils --doc
cargo test --features sync,async,unstable,unsafe --features test_utils --doc
cargo test --all-features --doc
```

Building a documentation:

```shell
cargo doc --no-deps --features sync,async,unstable,unsafe,derive,test_utils
```

<em>[← Implementation Notes](crate::docs::e2__implementation)</em>
 */
