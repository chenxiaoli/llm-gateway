# ---- Stage 1: Build frontend ----
FROM node:20-alpine AS frontend

WORKDIR /app/web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ ./
RUN npm run build

# ---- Stage 2: Prepare cargo-chef ----
FROM rust:1.94-slim AS chef
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef
WORKDIR /app

# ---- Stage 3: Plan (generate recipe.json) ----
FROM chef AS planner
COPY . .
COPY --from=frontend /app/web/dist ./web/dist
RUN cargo chef prepare --recipe-path recipe.json

# ---- Stage 4: Build dependencies (cached unless Cargo.toml changes) ----
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# ---- Stage 5: Build application ----
FROM chef AS build
COPY . .
COPY --from=frontend /app/web/dist ./web/dist
COPY --from=builder /app/target /app/target
RUN cargo build --release
RUN cp target/release/llm-gateway /app/llm-gateway

# ---- Stage 6: Runtime ----
FROM gcr.io/distroless/cc-debian12 AS runtime

WORKDIR /app
COPY --from=build /app/llm-gateway /app/llm-gateway
COPY config.toml /app/config.toml

USER 1000:1000

EXPOSE 8080

CMD ["/app/llm-gateway"]
