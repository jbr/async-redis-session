# async-redis-session
## redis-backed session store for [async-session](https://github.com/http-rs/async-session)

*Note the upstream version of this project is abandonware. This fork is intended to keep it alive and up to date.*

* [CI ![CI][ci-badge]][ci]
* [API Docs][docs] [![docs.rs docs][docs-badge]][docs]
* [Releases][releases] [![crates.io version][version-badge]][lib-rs]
* [Contributing][contributing]

[ci]: https://github.com/jbr/async-redis-session/actions?query=workflow%3ACI
[ci-badge]: https://github.com/jbr/async-redis-session/workflows/CI/badge.svg
[releases]: https://github.com/jbr/async-redis-session/releases
[![coverage](https://codecov.io/gh/valorem-labs-inc/async-redis-session/branch/main/graph/badge.svg?token=8W5MEJQSW6)](https://codecov.io/gh/valorem-labs-inc/async-redis-session)

## Installation
```sh
$ cargo add async-redis-session
```

## Safety
This crate uses ``#![deny(unsafe_code)]`` to ensure everything is implemented in
100% Safe Rust.

## License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br/>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>
