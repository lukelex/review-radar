FROM rust:1.90-bookworm AS builder
WORKDIR /app
RUN rustup component add clippy rustfmt
COPY Cargo.toml Cargo.lock ./
COPY crates/domain/Cargo.toml crates/domain/Cargo.toml
COPY crates/github/Cargo.toml crates/github/Cargo.toml
COPY crates/platform/Cargo.toml crates/platform/Cargo.toml
COPY crates/state/Cargo.toml crates/state/Cargo.toml
RUN mkdir -p crates/domain/src crates/github/src/bin crates/platform/src crates/state/src/bin \
    && echo '' > crates/domain/src/lib.rs \
    && echo '' > crates/platform/src/lib.rs \
    && echo '' > crates/state/src/lib.rs \
    && echo 'fn main() {}' > crates/state/src/bin/state.rs \
    && echo 'fn main() {}' > crates/github/src/main.rs \
    && echo 'fn main() {}' > crates/github/src/bin/seed_export.rs \
    && echo 'fn main() {}' > crates/github/src/bin/queue.rs \
    && cargo build --release --locked -p review-radar-github \
    && cargo build --release --locked -p review-radar-state --bin review-radar-state
COPY crates/github/src crates/github/src
COPY crates/domain/src crates/domain/src
COPY crates/platform/src crates/platform/src
COPY crates/state/src crates/state/src
COPY crates/github/tests crates/github/tests
COPY tests/fixtures tests/fixtures
RUN touch crates/domain/src/lib.rs crates/github/src/main.rs crates/github/src/bin/seed_export.rs crates/github/src/bin/queue.rs crates/platform/src/lib.rs crates/state/src/lib.rs crates/state/src/bin/state.rs \
    && cargo build --release --locked -p review-radar-github \
    && cargo build --release --locked -p review-radar-state --bin review-radar-state

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 1000 review-radar
COPY --from=builder /app/target/release/review-radar-github /usr/local/bin/review-radar-github
COPY --from=builder /app/target/release/review-radar-seed-export /usr/local/bin/review-radar-seed-export
COPY --from=builder /app/target/release/review-radar-queue /usr/local/bin/review-radar-queue
COPY --from=builder /app/target/release/review-radar-state /usr/local/bin/review-radar-state
USER review-radar
WORKDIR /app
ENTRYPOINT ["review-radar-github"]
CMD ["--database", "/data/review-radar.sqlite3"]
