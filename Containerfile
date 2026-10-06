FROM rust:1.99-trixie@sha256:15ad267e7a4cb2dce5905c90c76765adb6714945c5ea6d7c82673897a5e4067b AS builder

WORKDIR /build

# Cache dependencies in their own layer: build against stub targets first.
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
RUN mkdir src && \
    echo 'fn main() {}' > src/main.rs && \
    touch src/lib.rs && \
    cargo fetch --locked && \
    cargo build --release --locked && \
    rm -rf src

COPY src ./src
COPY .sqlx ./.sqlx
COPY migrations ./migrations

# Queries are checked against the committed .sqlx cache, not a live database.
ENV SQLX_OFFLINE=true
RUN touch src/main.rs src/lib.rs && \
    cargo build --release --locked --bin fee-manager

# cc, not static: rust links glibc by default. The Debian release must track
# the builder's — a trixie builder produces binaries that will not start on
# cc-debian12 (glibc 2.36).
FROM gcr.io/distroless/cc-debian13:nonroot@sha256:e792ab3d241a468a4fd7519ddbbebe66b49b5f365771716ea688ad40b6c6f1c2

WORKDIR /app

COPY --from=builder /build/target/release/fee-manager /app/fee-manager

# Numeric, not the `nonroot` name: Kubernetes runAsNonRoot validation cannot
# resolve a username.
USER 65532:65532

EXPOSE 3000

ENTRYPOINT ["/app/fee-manager"]
