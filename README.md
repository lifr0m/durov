# Description

This library is a Telegram MTProto high-level client.

Use `durov-client` crate as the main crate. See examples.

Project includes default implementation of telethon v7 session storage.
So sessions created by this project and telethon are compatible.

# Features

- Concurrent by nature, shareable between tasks. `Client` implements `Clone`.
- Optimized for very high-load bots. MTProto packing/unpacking is parallelized using all cpu cores.
- Minimum allocations and copying. Library uses its own contiguous deque buffer.
- Project complies with all [Telegram Security Guidelines](https://corefork.telegram.org/mtproto/security_guidelines).

# Todo

- Working with files:
    - https://corefork.telegram.org/api/optimisation#downloading-files-and-uploading-data-to-the-server
    - https://corefork.telegram.org/api/files
    - https://corefork.telegram.org/api/file-references#file-sources

# Security

Library uses [RustCrypto](https://github.com/rustcrypto) crates for all cryptography related operations.

`rand` crate with default `ThreadRng` engine is used as CSPRNG.
