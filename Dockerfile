# Build frontend
FROM node:18-alpine AS web_image

WORKDIR /build

COPY package.json ./
RUN npm install --force

COPY . .
RUN npm run build-only

# Build backend
FROM rust:1.85-alpine AS server_image

WORKDIR /build/backend

COPY ./backend/Cargo.toml ./backend/Cargo.lock ./
COPY ./backend/src ./src

RUN apk add --no-cache musl-dev openssl-dev pkgconfig
RUN cargo build --release

# run_image
FROM alpine:latest

WORKDIR /app

RUN apk add --no-cache bash ca-certificates tzdata

COPY --from=web_image /build/dist /app/web
COPY --from=server_image /build/backend/target/release/yt-panel-rust-backend /app/yt-panel
COPY backend/config/docker.toml /app/conf/app.toml

RUN mkdir -p /app/conf /app/database /app/uploads /app/web

ENV YT_PANEL_CONFIG=/app/conf/app.toml
ENV RUST_LOG=info

EXPOSE 3002

CMD ["/app/yt-panel"]
