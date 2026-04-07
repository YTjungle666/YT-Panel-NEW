# 部署指南

## Docker 部署

以下 Docker 镜像和 CT 模板均适用 [PolyForm Noncommercial License 1.0.0](LICENSE)，不允许商业用途。

```bash
# 先构建前端和 musl 后端产物
npm ci
npm run build-only
(cd backend && cargo build --release --target x86_64-unknown-linux-musl)

# 再打 Alpine 运行镜像
docker build -t yt-panel:alpine .

# 运行
docker run -d -p 80:80 \
  -v $(pwd)/data/database:/app/database \
  -v $(pwd)/data/uploads:/app/uploads \
  --name yt-panel \
  yt-panel:alpine
```

## PVE LXC 容器部署

```bash
# 在任意 Docker 主机上导出与 Docker 同源的 Alpine CT 模板
npm ci
npm run build-only
(cd backend && cargo build --release --target x86_64-unknown-linux-musl)
docker build -t yt-panel:alpine .
./scripts/export-pve-template.sh yt-panel:alpine ./artifacts/YT-Panel-NEW/release/yt-panel-alpine-ct-template.tar.zst

# 复制到 PVE 宿主机
scp ./artifacts/YT-Panel-NEW/release/yt-panel-alpine-ct-template.tar.zst \
  root@pve:/var/lib/vz/template/cache/

# 在 PVE 宿主机上创建 CT，无需再进入容器补环境
pct create 100 local:vztmpl/yt-panel-alpine-ct-template.tar.zst \
  --ostype unmanaged \
  --hostname yt-panel \
  --memory 1024 \
  --cores 2 \
  --rootfs local-lvm:4 \
  --net0 name=eth0,bridge=vmbr0,ip=dhcp \
  --unprivileged 0 \
  --start 1

# 验证
pct exec 100 -- wget -qO- http://127.0.0.1/ping
```

## 升级

```bash
# 保留数据，只更新代码
docker pull ghcr.io/ytjungle666/yt-panel-new:latest
docker stop yt-panel
docker rm yt-panel
docker run -d -p 80:80 \
  -v $(pwd)/data/database:/app/database \
  -v $(pwd)/data/uploads:/app/uploads \
  --name yt-panel \
  ghcr.io/ytjungle666/yt-panel-new:latest
```
