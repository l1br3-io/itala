default: lint test

ci: lint test deny doc

fmt:
    cargo fmt --all

lint:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets --all-features -- -D warnings

test:
    cargo nextest run --workspace --no-tests=pass
    cargo test --doc --workspace

deny:
    cargo deny check

doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace
