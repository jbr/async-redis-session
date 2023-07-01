# async-redis-session
## redis-backed session store for [async-session](https://github.com/http-rs/async-session)

[![CI][ci-badge]][ci] [![coverage](https://codecov.io/gh/valorem-labs-inc/async-redis-session/branch/main/graph/badge.svg?token=8W5MEJQSW6)](https://codecov.io/gh/valorem-labs-inc/async-redis-session)

[ci]: https://github.com/valorem-labs-inc/async-redis-session/actions?query=workflow%3ACI
[ci-badge]: https://github.com/valorem-labs-inc/async-redis-session/workflows/CI/badge.svg

*Note the upstream version of this project is abandonware. This fork is intended to keep it alive and up to date.*

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
