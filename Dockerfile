# Build frontend
FROM node:18-alpine AS web_image

RUN npm config set registry https://registry.npmmirror.com

WORKDIR /build

COPY package.json ./
RUN npm install

COPY . .
RUN npm run build-only

# Build backend
FROM rust:1.85-alpine AS server_image

WORKDIR /build/backend

COPY ./backend/Cargo.toml ./backend/Cargo.lock ./
COPY ./backend/src ./src

RUN apk add --no-cache musl-dev openssl-dev pkgconfig
RUN cargo build --release

# run_image - 最新Alpine，同时支持Docker和LXC/CT
FROM alpine:latest

WORKDIR /app

RUN apk add --no-cache bash ca-certificates tzdata

# 从构建阶段复制文件
COPY --from=web_image /build/dist /app/web
COPY --from=server_image /build/backend/target/release/yt-panel-rust-backend /app/yt-panel
COPY backend/config/docker.toml /app/conf/app.toml

# 创建必要目录
RUN mkdir -p /app/conf /app/database /app/uploads /app/web

ENV YT_PANEL_CONFIG=/app/conf/app.toml
ENV RUST_LOG=info

EXPOSE 3002

CMD ["/app/yt-panel"]
