# Description

This library is a Telegram MTProto high-level client.

Use `durov-client` crate as the main crate.

Project includes default implementation of telethon v7 session storage.
So sessions created by this project and telethon are compatible.

# Features

- Concurrent by nature, shareable between tasks. `Client` struct implements `Clone`
- No `unsafe` code
- Lock mechanisms are used only where necessary
- No needless bytes copying. Library uses its own contiguous deque buffer implementation to efficiently work with bytes
- Project complies with all [Telegram Security Guidelines](https://core.telegram.org/mtproto/security_guidelines)

# Feature flags

- `fast-buf` - enables faster implementation of `Buffer` which uses `unsafe` code

# Security

Library uses [RustCrypto](https://github.com/rustcrypto) crates for all cryptography related operations.

`rand` crate with default `ThreadRng` engine is used as CSPRNG.
