# Description

This library is a Telegram MTProto high-level client.

Use `durov-client` crate as the main crate.

# Features

- No `unsafe` code
- No lock mechanisms are used
- No needless bytes copies on sending and receiving. Library uses it's own contiguous deque buffer implementation to efficiently work with bytes. There are only 2 times when copy is occuring:
    1. TL object is serialized to buffer
    2. Buffer is written to the TCP stream
- Project complies with all [Telegram Security Guidelines](https://core.telegram.org/mtproto/security_guidelines)

# Feature flags

- `fast-buf` - enables faster implementation of `Buffer` which uses `unsafe` code
