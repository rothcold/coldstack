# Note: this project uses SQLite (bundled via rusqlite), not PostgreSQL.
# The single binary embeds both the React frontend and the Actix-web backend.

# ── Stage 1: build frontend ──────────────────────────────────────────────────
FROM node:22-alpine AS frontend
WORKDIR /build/frontend
COPY frontend/package*.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

# ── Stage 2: build Rust binary ───────────────────────────────────────────────
FROM rust:1-slim-bookworm AS builder
# rusqlite "bundled" compiles SQLite from C source — needs a C toolchain
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Cache Cargo dependencies before copying real source.
# rust-embed requires frontend/dist to exist at compile time, so we provide
# a minimal stub to avoid breaking the embed macro during the cache layer.
COPY backend/Cargo.toml backend/Cargo.lock ./backend/
RUN mkdir -p frontend/dist && touch frontend/dist/.keep
RUN mkdir -p backend/src && echo 'fn main() {}' > backend/src/main.rs
WORKDIR /build/backend
RUN cargo build --release || true
RUN rm -rf src target/release/.fingerprint/coldstack-server-*

# Now build for real
WORKDIR /build
COPY backend/src ./backend/src
COPY --from=frontend /build/frontend/dist ./frontend/dist
WORKDIR /build/backend
RUN cargo build --release

# ── Stage 3: minimal runtime ─────────────────────────────────────────────────
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/backend/target/release/coldstack /usr/local/bin/coldstack

# tasks.db is created at startup in the working directory.
# Mount a volume here so the database survives container restarts.
WORKDIR /data
VOLUME ["/data"]

EXPOSE 8080
CMD ["coldstack"]
