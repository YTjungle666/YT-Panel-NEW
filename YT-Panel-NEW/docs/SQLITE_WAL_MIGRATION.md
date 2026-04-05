# SQLite WAL 模式迁移指南（方案 B）

## 📋 概述
本文档指导如何在维护窗口期手动启用 SQLite WAL（Write-Ahead Logging）模式，提升数据库并发性能。

**适用版本**: YT-Panel-NEW v1.0+
**风险等级**: ⭐⭐⭐ 中等（需停机维护）
**预计耗时**: 5-10 分钟
**性能提升**: 并发读取性能提升 50-200%

---

## 🎯 为什么需要 WAL 模式？

### 默认模式（DELETE）的问题
- 写入时锁定整个数据库文件
- 并发读取需要等待写入完成
- 高并发场景下性能瓶颈明显

### WAL 模式的优势
- 写入操作追加到 WAL 文件，不阻塞读取
- 支持**多读单写**并发
- 读取性能显著提升（无锁竞争）

```
DELETE 模式: [写入] → [锁定文件] → [其他读取等待]
WAL 模式:    [读取A] ← [读取B] ← [WAL文件] ← [写入]
              ↑______________↑
                  并发进行
```

---

## ⚠️ 前置条件与风险

### 必须完成
- [ ] 停止 YT-Panel 服务（`systemctl stop yt-panel` 或 Docker 停止）
- [ ] 备份数据库文件（**重要！**）
- [ ] 确认维护窗口期（建议低峰期操作）

### 风险提示
- WAL 模式会创建额外的 `-wal` 和 `-shm` 文件
- 如果 WAL 文件过大（>100MB），会自动 checkpoint
- 极端情况下（磁盘满）可能导致数据不一致

---

## 🔧 操作步骤

### 1. 停止服务
```bash
# Systemd 方式
sudo systemctl stop yt-panel

# Docker 方式
docker-compose stop backend

# 或直接停止容器
docker stop yt-panel-backend
```

### 2. 备份数据库
```bash
# 定位数据库文件（默认路径）
DB_PATH="./backend/database/database.db"
BACKUP_DIR="./backups/$(date +%Y%m%d_%H%M%S)"

# 创建备份目录
mkdir -p "$BACKUP_DIR"

# 完整备份
cp "$DB_PATH" "$BACKUP_DIR/database.db.bak"

# 验证备份
ls -lh "$BACKUP_DIR/"
```

### 3. 启用 WAL 模式
```bash
# 进入数据库目录
cd ./backend/database

# 使用 sqlite3 CLI 执行（必须安装 sqlite3）
sqlite3 database.db "
    -- 启用 WAL 模式
    PRAGMA journal_mode=WAL;
    
    -- 设置同步级别（性能与安全的平衡）
    PRAGMA synchronous=NORMAL;
    
    -- 设置缓存大小（10MB，可根据内存调整）
    PRAGMA cache_size=10000;
    
    -- 验证设置
    SELECT * FROM pragma_journal_mode;
"
```

**预期输出**：
```
wal
```

### 4. 验证迁移成功
```bash
# 检查生成的文件
ls -lh ./backend/database/

# 预期看到新文件：
# database.db
# database.db-wal      ← WAL 日志文件
# database.db-shm      ← 共享内存文件

# 再次验证模式
sqlite3 database.db "PRAGMA journal_mode;"
# 输出: wal
```

### 5. 启动服务
```bash
# Systemd
sudo systemctl start yt-panel

# Docker
docker-compose up -d backend
```

### 6. 功能验证
```bash
# 检查服务日志
journalctl -u yt-panel -f
# 或
docker logs -f yt-panel-backend

# 测试关键功能：
# 1. 用户登录
# 2. 书签添加/删除
# 3. 文件上传
```

---

## 🔄 回滚方法（如果出现问题）

如果启用 WAL 后遇到问题，可立即回滚：

```bash
# 停止服务
sudo systemctl stop yt-panel

# 恢复 DELETE 模式
sqlite3 ./backend/database/database.db "PRAGMA journal_mode=DELETE;"

# 删除 WAL 相关文件（可选，会自动清理）
rm -f ./backend/database/database.db-wal
rm -f ./backend/database/database.db-shm

# 如需恢复备份（极端情况）
cp ./backups/YYYYMMDD_HHMMSS/database.db.bak ./backend/database/database.db

# 启动服务
sudo systemctl start yt-panel
```

---

## 📊 验证性能提升

### 监控指标
迁移后观察以下指标：

```bash
# 查看 WAL 文件大小（定期执行）
ls -lh ./backend/database/*.db-wal

# 检查 checkpoint 统计
sqlite3 database.db "PRAGMA wal_checkpoint;"

# 查看 WAL 模式状态
sqlite3 database.db "
    SELECT 
        * FROM pragma_journal_mode;
    SELECT 
        * FROM pragma_wal_checkpoint;
"
```

### 性能对比测试
```bash
# 使用 wrk 或 ab 进行并发测试
# 迁移前记录基准，迁移后对比

# 示例：测试书签列表 API
wrk -t4 -c100 -d30s http://localhost:3000/api/panel/bookmark/getList
```

---

## 🔍 故障排查

### 问题 1：WAL 文件过大
```sql
-- 手动触发 checkpoint（合并 WAL 到主数据库）
PRAGMA wal_checkpoint(TRUNCATE);
```

### 问题 2：WAL 模式未生效
```sql
-- 检查是否有其他连接锁定数据库
-- 确保没有只读连接在启用 WAL 前打开
```

### 问题 3：权限错误
```bash
# 确保数据库文件权限正确
chown -R 1000:1000 ./backend/database/
chmod 664 ./backend/database/database.db
```

---

## 📝 自动化脚本

以下是完整的一键迁移脚本（保存为 `enable_wal.sh`）：

```bash
#!/bin/bash
set -e

DB_PATH="${1:-./backend/database/database.db}"
BACKUP_DIR="./backups/$(date +%Y%m%d_%H%M%S)"

echo "🛑 步骤 1/5: 检查数据库文件..."
if [ ! -f "$DB_PATH" ]; then
    echo "❌ 错误: 数据库文件不存在: $DB_PATH"
    exit 1
fi

echo "💾 步骤 2/5: 备份数据库到 $BACKUP_DIR..."
mkdir -p "$BACKUP_DIR"
cp "$DB_PATH" "$BACKUP_DIR/database.db.bak"
echo "✅ 备份完成: $BACKUP_DIR/database.db.bak"

echo "🔧 步骤 3/5: 启用 WAL 模式..."
sqlite3 "$DB_PATH" "
    PRAGMA journal_mode=WAL;
    PRAGMA synchronous=NORMAL;
    PRAGMA cache_size=10000;
"

echo "🔍 步骤 4/5: 验证..."
MODE=$(sqlite3 "$DB_PATH" "PRAGMA journal_mode;")
if [ "$MODE" = "wal" ]; then
    echo "✅ WAL 模式启用成功!"
else
    echo "❌ WAL 模式启用失败，当前模式: $MODE"
    exit 1
fi

echo "📊 步骤 5/5: 当前数据库文件..."
ls -lh "$(dirname "$DB_PATH")"/*.db*

echo ""
echo "🎉 迁移完成！请启动服务并验证功能。"
echo "📝 如需回滚: sqlite3 $DB_PATH 'PRAGMA journal_mode=DELETE;'"
```

**使用方式**：
```bash
chmod +x enable_wal.sh
./enable_wal.sh ./backend/database/database.db
```

---

## 🏷️ 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 1.0 | 2024-01-XX | 初始版本 |

---

## 📚 参考链接

- [SQLite WAL Mode Documentation](https://www.sqlite.org/wal.html)
- [SQLite PRAGMA journal_mode](https://www.sqlite.org/pragma.html#pragma_journal_mode)
