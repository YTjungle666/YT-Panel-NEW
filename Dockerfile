# 最终运行镜像 - 产物最小化
FROM alpine

WORKDIR /app

RUN apk add --no-cache bash ca-certificates tzdata

# 从构建产物复制文件
COPY dist /app/web
COPY yt-panel-rust-backend /app/yt-panel
COPY docker.toml /app/conf/app.toml

# 创建必要目录
RUN mkdir -p /app/conf /app/database /app/uploads /app/web

ENV YT_PANEL_CONFIG=/app/conf/app.toml
ENV RUST_LOG=info

EXPOSE 3002

CMD ["/app/yt-panel"]
