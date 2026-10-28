# syntax=docker/dockerfile:1

FROM node:24-bookworm-slim AS frontend-build
WORKDIR /build/frontend
RUN corepack enable && corepack prepare pnpm@11.8.0 --activate
COPY frontend/package.json frontend/pnpm-lock.yaml frontend/pnpm-workspace.yaml ./
RUN --mount=type=cache,target=/root/.local/share/pnpm/store pnpm install --frozen-lockfile
COPY frontend/ ./
RUN pnpm build

FROM rust:1.96.1-bookworm AS backend-build
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY backend/ ./backend/
COPY entwicklungsuebersicht_mathe_DRUCKVERSION_270925.pdf ./
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --release --locked && install -D target/release/tamara /out/tamara

FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN mkdir -p /data && chown 10001:10001 /data
COPY --from=backend-build /out/tamara /usr/local/bin/tamara
COPY --from=backend-build /build/backend/assets/DejaVu-LICENSE.txt /usr/share/licenses/tamara/DejaVu-LICENSE.txt
COPY --from=frontend-build /build/frontend/dist/ /app/frontend/
ENV TAMARA_BIND=0.0.0.0:3000 \
    TAMARA_FRONTEND_DIR=/app/frontend \
    DATABASE_URL=sqlite:/data/tamara.sqlite
USER 10001:10001
VOLUME ["/data"]
EXPOSE 3000
CMD ["/usr/local/bin/tamara"]
