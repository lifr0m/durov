# Description

This library is a Telegram MTProto high-level client.

Use `durov-client` crate as the main crate. See examples.

# Features

- Concurrent by nature, shareable between tasks. `Client` implements `Clone`.
- Minimum allocations and copying. Library uses its own contiguous deque buffer.
- Project complies with all [Telegram Security Guidelines](https://corefork.telegram.org/mtproto/security_guidelines).

# Feature flags

- `session-telethon` - telethon v7 session storage.

# Security

Library uses [RustCrypto](https://github.com/rustcrypto) crates for all cryptography related operations.

`rand` crate with default `ThreadRng` engine is used as CSPRNG.
