FROM alpine:3.23

WORKDIR /app

RUN apk add --no-cache ca-certificates \
    && mkdir -p /app/conf /app/database /app/uploads /app/web

ENV YT_PANEL_CONFIG=/app/conf/app.toml

COPY dist /app/web
COPY backend/target/x86_64-unknown-linux-musl/release/yt-panel-rust-backend /app/yt-panel
COPY scripts/ct-entrypoint.sh /app/ct-entrypoint.sh
COPY backend/config/docker.toml /app/conf/app.toml

RUN chmod +x /app/ct-entrypoint.sh

EXPOSE 80

CMD ["/app/yt-panel"]
