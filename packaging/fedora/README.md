# Fedora packaging

Lands in T-32 (packaging and distribution). Until then this directory
only anchors the monorepo layout from
[docs/design/01-architecture.md](../../docs/design/01-architecture.md).

The Fedora package will carry the pinned toolchain expectations from
the repository root: Rust per `rust-toolchain.toml`, Qt 6.11 per the
top-level `CMakeLists.txt`, Smithay `=0.7.0` per the root `Cargo.toml`.
