# Stage 1: Build React + Vite + Tailwind SPA
FROM node:22-slim AS web-builder
WORKDIR /app/web
RUN corepack enable && corepack prepare pnpm@11.9.0 --activate
COPY web/package.json web/pnpm-lock.yaml web/pnpm-workspace.yaml ./
RUN pnpm install --frozen-lockfile
COPY web/ ./
RUN pnpm build

# Stage 2: Build Rust release binary (pum-api)
FROM rust:1-bookworm AS rust-builder
WORKDIR /app
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY migrations ./migrations
COPY crates ./crates
RUN cargo build --release --locked -p pum-api

# Stage 3: Minimal production runtime image (non-root)
FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -m -u 10001 appuser

WORKDIR /app
COPY --from=rust-builder /app/target/release/pum-api /app/pum-api
COPY --from=web-builder /app/web/dist /app/web/dist
RUN chown -R appuser:appuser /app

USER appuser

ENV APP_HOST=0.0.0.0
ENV PORT=8080
ENV COOKIE_SECURE=true
ENV RUST_LOG=info,pum_api=info

EXPOSE 8080
CMD ["/app/pum-api"]
