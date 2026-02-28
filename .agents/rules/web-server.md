---
trigger: always_on
glob: "**/*.rs"
description: Web Server coding guidelines for idiomatic Rust.
---

# Web Server Guidelines

## Idiomatic Rust
- Write clean and idiomatic Rust code. Use modern Rust features up to edition 2021.
- Make liberal use of `std::result::Result` and `std::option::Option`. 

## Strictly No Unwrap
- **NEVER** use `.unwrap()`. It is strictly prohibited in this codebase, including tests.
- Always use proper error handling. If a panic is truly intended or impossible to avoid in test setups, use `.expect("descriptive message")` instead of `.unwrap()` so the reason is documented.
- Use the `?` operator for propagating errors wherever possible.
- Adhere to `clippy::unwrap_used` rules.
