# Stage 1: Builder
FROM rust:1.91 AS builder

WORKDIR /build

# Copy manifests first for better layer caching
COPY Cargo.toml Cargo.lock ./

# Create dummy main to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    echo "pub fn dummy() {}" > src/lib.rs && \
    cargo build --release && \
    rm -rf src

# Copy actual source code
COPY src ./src
COPY .sqlx ./.sqlx
COPY migrations ./migrations

# Build the actual application
ENV SQLX_OFFLINE=true
RUN touch src/main.rs src/lib.rs && \
    cargo build --release --bin fee-manager

# Stage 2: Runtime
FROM gcr.io/distroless/cc-debian12:nonroot

WORKDIR /app

COPY --from=builder /build/target/release/fee-manager /app/fee-manager

EXPOSE 3000

CMD ["/app/fee-manager"]
