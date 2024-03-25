/*!
# 📖 4.1. Guidelines

<em>[← Ad-hoc Dialects](crate::docs::c4__ad_hoc_dialects) | [Implementation Notes →](crate::docs::e2__implementation)</em>

## Contents

1. [General Considerations](#general-considerations)
1. [Documentation](#documentation)
1. [API Changes](#api-changes)
1. [`no_std` Functionality](#no_std-functionality)
1. [Code Generation](#code-generation)

## General Considerations

In this project we are following [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
as close as possible. There are important amendments mainly related to
[Flexibility](https://rust-lang.github.io/api-guidelines/flexibility.html). For example, we do not
follow [`C-INTERMEDIATE`](https://rust-lang.github.io/api-guidelines/flexibility.html#functions-expose-intermediate-results-to-avoid-duplicate-work-c-intermediate)
since this library API is not yet sealed. But when it comes to the other norms, then the significant
deviation should be considered as a bug or (our) negligence and should be fixed.

When it comes to patterns and best practices, we prefer consistency and boredom over excitement and
diversity. Which means that we are not going to introduce something ad-hoc even if it looks right.
Beyond that, we are open for discussions and critique of our approaches. The main limitation is
that, given our limited resources, we are able to engage into argument only with those who are ready
to substantive their claims by contributing to this project.

We use [Clippy](https://doc.rust-lang.org/stable/clippy/) for linting and follow its suggestions. If
you've found places in our code, where we've made an exclusion to Clippy rules, and you can fix
this, we would be grateful to you.

## Documentation

We favor good documentation and commit to maintaining it in the proper state. Therefore, we don't
accept undocumented changes or changes with badly written documentation. At the same time,
pull-requests for fixing documentation bugs will have high priority.

We do not require to document private methods. However, we prefer documented private method with a
sensible naming over regular comments. In a perfect world we would probably replace all comments
with documented private methods.

The general workflow for documentation is adding docs and examples to methods and entities and only
then add them to the [`docs`].

At the moment we are using in-project documentation, and we like it this way since it is easier to
set up and to fail fast. But in the future we may move to
[mdBook](https://rust-lang.github.io/mdBook/).

## API Changes

All changes to [Asynchronous API](crate::docs::a4__async_api) should be reflected by changes to
[Synchronous API](crate::docs::a3__sync_api) and vice versa. Orphan changes to only one type of API
are not allowed. The main exception is adding a new kind of transport. For example, if we have a
good asynchronous library for message queue `X` but its synchronous version is not reliable or
requires significant efforts to integrate, then we are good to go. You've got the point.

We are encouraging all library users to submit [issues](https://gitlab.com/mavka/libs/maviola/-/issues)
and join us discussing them. This will help us to identify potential problems and preferred
use-cases, plan our [roadmap](https://gitlab.com/mavka/libs/maviola/-/milestones) properly, and
deprecate confusing or unnecessary features in a predictable manner.

## `no_std` Functionality

This library is strictly [`std`] but [Mavio](https://gitlab.com/mavka/libs/mavio) upon which Maviola
is built is both `no_std` and `no-alloc`. If you are working on potentially `no_std` feature,
consider to submit your changes into Mavio. We would be happy to help you to propagate your changes
downstream.

## Code Generation

The same can be said about [MAVSpec](https://gitlab.com/mavka/libs/mavspec) that we are using as a
code generator. If it is possible to achieve certain goals by tweaking code generation in a
non-breaking way, we are here to support and advise.

<em>[← Ad-hoc Dialects](crate::docs::c4__ad_hoc_dialects) | [Implementation Notes →](crate::docs::e2__implementation)</em>

[`docs`]: crate::docs
 */
