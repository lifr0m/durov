# Description

This library is a Telegram MTProto high-level client.

Use `durov-client` crate as the main crate.

Project includes default implementation of telethon v7 session storage.
So sessions created by this project and telethon are compatible.

# Features

- No `unsafe` code.
- Concurrent by nature, shareable between tasks. `Client` implements `Clone`.
- Convenient work with RPC errors (`Error::is`, `Error::message`, `Error::parse`).
- Lock mechanisms are used only where necessary.
- No needless bytes copying. Library uses its own contiguous deque buffer implementation to efficiently work with bytes.
  Serialization happens only at the end to the buffer which is written directly to the stream.
- Project complies with all [Telegram Security Guidelines](https://core.telegram.org/mtproto/security_guidelines).

# Feature flags

- `fast-buf` - enables faster implementation of `Buffer` which uses `unsafe` code.

# Todo

- Working with files:
    - https://corefork.telegram.org/api/optimisation#downloading-files-and-uploading-data-to-the-server
    - https://corefork.telegram.org/api/files
    - https://corefork.telegram.org/api/file-references#file-sources

# Security

Library uses [RustCrypto](https://github.com/rustcrypto) crates for all cryptography related operations.

`rand` crate with default `ThreadRng` engine is used as CSPRNG.
