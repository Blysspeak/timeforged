# ============================================================
# Stage 1: Build Vue dashboard
# ============================================================
FROM node:22-alpine AS web-builder

WORKDIR /app/crates/timeforged/web
COPY crates/timeforged/web/package.json crates/timeforged/web/package-lock.json* ./
RUN npm install
COPY crates/timeforged/web/ ./
RUN npx vue-tsc --noEmit && npx vite build

# ============================================================
# Stage 2: Build Rust binaries (daemon + CLI)
# ============================================================
FROM rustlang/rust:nightly-alpine AS rust-builder

RUN apk add --no-cache musl-dev pkgconfig openssl-dev

WORKDIR /app

# Cache dependencies by copying manifests first
COPY Cargo.toml Cargo.lock* ./
COPY crates/timeforged-core/Cargo.toml crates/timeforged-core/Cargo.toml
COPY crates/timeforged/Cargo.toml crates/timeforged/Cargo.toml
COPY crates/tf/Cargo.toml crates/tf/Cargo.toml

# Create dummy sources so cargo can resolve the workspace
RUN mkdir -p crates/timeforged-core/src && echo "pub mod config; pub mod error; pub mod models; pub mod api;" > crates/timeforged-core/src/lib.rs \
    && mkdir -p crates/timeforged/src && echo "fn main(){}" > crates/timeforged/src/main.rs \
    && mkdir -p crates/tf/src && echo "fn main(){}" > crates/tf/src/main.rs

# Pre-build dependencies
RUN cargo build --release 2>/dev/null || true

# Copy real source code
COPY crates/ crates/

# Copy built web assets into the embed directory
COPY --from=web-builder /app/crates/timeforged/web/dist crates/timeforged/web/dist

# Touch main files to invalidate cache for the actual crates
RUN touch crates/timeforged-core/src/lib.rs crates/timeforged/src/main.rs crates/tf/src/main.rs

RUN cargo build --release

# ============================================================
# Stage 3: Minimal runtime image
# ============================================================
FROM alpine:3.21

# Copy ca-certificates from builder to avoid network issues during build
COPY --from=rust-builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt

RUN adduser -D -h /home/timeforged timeforged
USER timeforged

COPY --from=rust-builder /app/target/release/timeforged /usr/local/bin/timeforged
COPY --from=rust-builder /app/target/release/tf /usr/local/bin/tf

# Data and config directories
RUN mkdir -p /home/timeforged/.local/share/timeforged \
    && mkdir -p /home/timeforged/.config/timeforged

ENV TF_HOST=0.0.0.0
ENV TF_PORT=6175
ENV TF_DATABASE_URL=sqlite:/home/timeforged/.local/share/timeforged/timeforged.db?mode=rwc
ENV TF_LOG_LEVEL=info
ENV TF_SERVER_URL=http://127.0.0.1:6175

EXPOSE 6175

VOLUME ["/home/timeforged/.local/share/timeforged"]

CMD ["timeforged"]
