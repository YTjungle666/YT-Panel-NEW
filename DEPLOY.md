# 部署指南

## Docker 部署

```bash
# 构建镜像
docker build -t yt-panel .

# 运行
docker run -d -p 3000:3000 -p 3001:3001 \
  -v $(pwd)/data:/app/data \
  --name yt-panel \
  yt-panel
```

## PVE LXC 容器部署

```bash
# 在 PVE 宿主机上执行
pct create 100 /var/lib/vz/template/cache/debian-12-standard.tar.zst \
  --ostype debian \
  --hostname yt-panel \
  --memory 1024 \
  --cores 2 \
  --net0 name=eth0,bridge=vmbr0,ip=dhcp

pct start 100
pct exec 100 -- bash -c "
  apt update && apt install -y curl git
  curl -fsSL https://deb.nodesource.com/setup_20.x | bash -
  apt install -y nodejs cargo
"

# 复制项目到容器
pct push 100 ./yt-panel-new /opt/yt-panel/

pct exec 100 -- bash -c "
  cd /opt/yt-panel
  ./build.sh
  ./start.sh
"
```

## 升级

```bash
# 保留数据，只更新代码
docker pull yt-panel:latest
docker stop yt-panel
docker rm yt-panel
docker run -d -p 3000:3000 -p 3001:3001 \
  -v $(pwd)/data:/app/data \
  --name yt-panel \
  yt-panel:latest
```
