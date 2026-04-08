# git-ops

## After Each Change
Run the following commands after every code change and fix any issues before considering the change complete:

1. `cargo fmt` - Format all code
2. `cargo test` - Run all tests
3. `cargo clippy` - Run linter; fix all warnings and errors before completing the change

## Dependencies
Always use exact versions for dependencies in `Cargo.toml` (e.g., `"4.5.60"` not `"4"`). Check `Cargo.lock` for the resolved version when pinning.

## Module Conventions
Never use `mod.rs`. Always use the modern Rust style: create a top-level file (e.g., `foo.rs`) as the module root, and a matching folder (`foo/`) for any submodules.

## Import Style
Always import symbols at the top of the file with `use` statements. Never call items using their full `crate::` namespace inline in function bodies or signatures. For example:

```rust
// Correct
use crate::feature::task::listing;
listing::list_open_tasks(...)

// Wrong
crate::feature::task::listing::list_open_tasks(...)
```

When a name would clash, it's okay to use the full `crate::` namespace call inline rather than aliasing.
