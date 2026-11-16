# UNO 多階段建置（PLAN T11.5）：WASM + Web + Rust server → slim runtime
# 執行：docker compose up -d（host 網路，手機掃 QR 加入）

FROM rust:trixie AS wasm
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY scripts ./scripts
RUN rustup target add wasm32-unknown-unknown \
    && ARCH="$(uname -m)" \
    && curl -sL "https://github.com/rustwasm/wasm-pack/releases/download/v0.13.1/wasm-pack-v0.13.1-${ARCH}-unknown-linux-musl.tar.gz" -o /tmp/wp.tgz \
    && tar -xzf /tmp/wp.tgz -C /tmp \
    && cp "/tmp/wasm-pack-v0.13.1-${ARCH}-unknown-linux-musl/wasm-pack" /usr/local/bin/ \
    && ./scripts/build-wasm.sh

FROM node:22-trixie-slim AS web
WORKDIR /app
RUN corepack enable && corepack prepare pnpm@12.16.1 --activate
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY apps/web ./apps/web
COPY --from=wasm /app/apps/web/src/wasm/pkg ./apps/web/src/wasm/pkg
RUN pnpm install --frozen-lockfile && pnpm -C apps/web build

FROM rust:trixie AS server
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo build -p uno-server --release

FROM debian:trixie-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && mkdir -p /app/models /data
COPY --from=server /app/target/release/uno-server /usr/local/bin/uno-server
COPY --from=web /app/apps/web/dist /app/web
WORKDIR /app
EXPOSE 8787
ENTRYPOINT ["uno-server", "--host", "0.0.0.0", "--port", "8787", "--web-dir", "/app/web", "--models-dir", "/app/models", "--data-dir", "/data"]
