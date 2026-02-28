---
description: "Web Server Development Workflow"
---

# Web Server Workflow

This workflow outlines the standard development process for the Web Server Report project.

## 1. Development
1. Write idiomatic Rust code.
2. Adhere to the strict "no unwrap()" policy. Use `.expect()` with a descriptive message if panics are unavoidable in tests.

## 2. Testing and Linting
1. Run all tests to verify correctness:
```bash
// turbo
cargo test
```

2. Run clippy to ensure no warnings or forbidden patterns (like `unwrap()`):
```bash
// turbo
cargo clippy --all-targets -- -D warnings -D clippy::unwrap_used
```

## 3. Building
1. Build the project:
```bash
// turbo
cargo build
```
