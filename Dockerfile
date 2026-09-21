FROM rust:1.90-bookworm AS builder
WORKDIR /app
RUN rustup component add rustfmt
COPY Cargo.toml Cargo.lock ./
COPY crates/domain/Cargo.toml crates/domain/Cargo.toml
COPY crates/github/Cargo.toml crates/github/Cargo.toml
RUN mkdir -p crates/domain/src crates/github/src/bin \
    && echo '' > crates/domain/src/lib.rs \
    && echo 'fn main() {}' > crates/github/src/main.rs \
    && echo 'fn main() {}' > crates/github/src/bin/seed_export.rs \
    && cargo build --release --locked -p review-radar-github
COPY crates/github/src crates/github/src
COPY crates/domain/src crates/domain/src
COPY crates/github/tests crates/github/tests
COPY tests/fixtures tests/fixtures
RUN touch crates/domain/src/lib.rs crates/github/src/main.rs crates/github/src/bin/seed_export.rs \
    && cargo build --release --locked -p review-radar-github

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 1000 review-radar
COPY --from=builder /app/target/release/review-radar-github /usr/local/bin/review-radar-github
COPY --from=builder /app/target/release/review-radar-seed-export /usr/local/bin/review-radar-seed-export
USER review-radar
WORKDIR /app
ENTRYPOINT ["review-radar-github"]
CMD ["--database", "/data/review-radar.sqlite3"]
