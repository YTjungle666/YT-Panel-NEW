# Build frontend
FROM node:18-alpine AS frontend-builder
WORKDIR /src
COPY package*.json ./
RUN npm ci
COPY . .
RUN YT_PANEL_DIST_OUT_DIR=/out/frontend-dist npm run build

# Build backend  
FROM rust:1.85-alpine AS backend-builder
RUN apk add --no-cache musl-dev openssl-dev
WORKDIR /src/backend
COPY backend/Cargo.toml backend/Cargo.lock ./
COPY backend/src ./src
RUN cargo build --release --locked

# Runtime - 参考 sun-panel-v2，使用默认 alpine
FROM alpine
WORKDIR /app
RUN apk add --no-cache bash ca-certificates tzdata

COPY --from=backend-builder /src/backend/target/release/yt-panel-rust-backend /app/yt-panel
COPY backend/config/docker.toml /app/conf/app.toml
COPY --from=frontend-builder /out/frontend-dist /app/web

RUN mkdir -p /app/conf /app/database /app/uploads /app/web \
    && chmod +x /app/yt-panel

ENV YT_PANEL_CONFIG=/app/conf/app.toml
EXPOSE 80

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget -qO- http://localhost:80/ping || exit 1

CMD ["/app/yt-panel"]
