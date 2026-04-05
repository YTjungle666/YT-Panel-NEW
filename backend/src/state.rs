use crate::config::AppConfig;
use lru::LruCache;
use sqlx::SqlitePool;
use std::{
    num::NonZeroUsize,
    sync::Arc,
    time::Instant,
};
use tokio::sync::RwLock;

/// 会话条目，包含持久化令牌和最后访问时间
#[derive(Clone, Debug)]
pub struct SessionEntry {
    pub persistent_token: String,
    pub last_accessed: Instant,
}

impl SessionEntry {
    pub fn new(persistent_token: String) -> Self {
        Self {
            persistent_token,
            last_accessed: Instant::now(),
        }
    }

    /// 更新最后访问时间
    pub fn touch(&mut self) {
        self.last_accessed = Instant::now();
    }
}

/// LRU 会话缓存容量（最大会话数）
const SESSION_CACHE_CAPACITY: usize = 1000;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub config: Arc<AppConfig>,
    /// 会话缓存：临时 token -> 会话条目
    /// 使用 LRU 自动淘汰最少使用的会话，限制内存占用
    pub sessions: Arc<RwLock<LruCache<String, SessionEntry>>>,
}

impl AppState {
    pub fn new(db: SqlitePool, config: Arc<AppConfig>) -> Self {
        // 创建固定容量的 LRU 缓存
        let capacity = NonZeroUsize::new(SESSION_CACHE_CAPACITY)
            .expect("Capacity must be non-zero");
        let sessions = LruCache::new(capacity);

        Self {
            db,
            config,
            sessions: Arc::new(RwLock::new(sessions)),
        }
    }

    /// 获取会话，同时更新访问时间（LRU 自动处理）
    pub async fn get_session(&self, token: &str) -> Option<SessionEntry> {
        let mut cache = self.sessions.write().await;
        // get 会自动更新 LRU 顺序
        cache.get(token).cloned()
    }

    /// 插入或更新会话
    pub async fn put_session(&self, token: String, entry: SessionEntry) {
        let mut cache = self.sessions.write().await;
        cache.put(token, entry);
    }

    /// 使会话失效
    pub async fn invalidate_session(&self, token: &str) {
        let mut cache = self.sessions.write().await;
        cache.pop(token);
    }

    /// 获取当前会话数量
    pub async fn session_count(&self) -> usize {
        let cache = self.sessions.read().await;
        cache.len()
    }
}
