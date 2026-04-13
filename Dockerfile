# ---- Stage 1: Build frontend ----
FROM node:20-alpine AS frontend

WORKDIR /app/web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ ./
RUN npm run build

# ---- Stage 2: Build Rust backend ----
FROM rust:1.83-alpine AS backend

RUN apk add --no-cache musl-dev pkgconf openssl-dev openssl-libs-static

RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

# Copy frontend dist for rust-embed
COPY --from=frontend /app/web/dist ./web/dist

# Build release binary (static linking)
RUN RUSTFLAGS='-C target-feature=+crt-static -C link-arg=-s' \
    cargo build --release --target x86_64-unknown-linux-musl && \
    cp target/x86_64-unknown-linux-musl/release/llm-gateway /app/llm-gateway

# ---- Stage 3: Runtime ----
FROM alpine:3.20

RUN apk add --no-cache ca-certificates tzdata && \
    addgroup -S gateway && adduser -S gateway -G gateway

WORKDIR /app

COPY --from=backend /app/llm-gateway .
COPY config.toml .

RUN mkdir -p /app/data && chown -R gateway:gateway /app

USER gateway

EXPOSE 8080

VOLUME ["/app/data"]

CMD ["./llm-gateway"]
