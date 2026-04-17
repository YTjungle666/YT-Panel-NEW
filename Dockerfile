# syntax=docker/dockerfile:1.7

ARG RUNTIME_IMAGE=alpine:3.23@sha256:25109184c71bdad752c8312a8623239686a9a2071e8825f20acb8f2198c3f659

FROM ${RUNTIME_IMAGE} AS runtime
WORKDIR /app

LABEL org.opencontainers.image.title="YT-Panel-NEW" \
      org.opencontainers.image.description="Rust-first self-hosted server and NAS dashboard" \
      org.opencontainers.image.source="https://github.com/YTjungle666/YT-Panel-NEW" \
      org.opencontainers.image.licenses="PolyForm-Noncommercial-1.0.0"

RUN apk add --no-cache ca-certificates libcap \
    && addgroup -S ytpanel \
    && adduser -S -G ytpanel -h /app ytpanel \
    && mkdir -p /app/conf /app/database /app/uploads /app/web

ENV YT_PANEL_CONFIG=/app/conf/app.toml

COPY LICENSE /app/LICENSE
COPY backend/config/docker.toml /app/conf/app.toml
COPY dist /app/web
COPY backend/target/x86_64-unknown-linux-musl/release/yt-panel-rust-backend /app/yt-panel-bin
COPY --chmod=755 scripts/container-init.sh /app/yt-panel

RUN ln -s /app/yt-panel /usr/local/bin/container-init \
    && chmod 0755 /app/yt-panel /app/yt-panel-bin \
    && chown -R ytpanel:ytpanel /app/database /app/uploads \
    && chown ytpanel:ytpanel /app/yt-panel /app/yt-panel-bin /app/LICENSE \
    && setcap cap_net_bind_service=+ep /app/yt-panel-bin

EXPOSE 80

HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
  CMD wget -qO- http://127.0.0.1/ping >/dev/null 2>&1 || exit 1

USER ytpanel:ytpanel

CMD ["/usr/local/bin/container-init"]

FROM runtime AS ct-template
USER root
RUN rm -f /sbin/init
COPY scripts/ct-entrypoint.sh /sbin/init
RUN chmod 0755 /sbin/init
