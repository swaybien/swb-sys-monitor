use crate::stats::{Result, SystemStats, collect_system_stats};
use std::sync::Arc;
use std::sync::atomic::{AtomicPtr, AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// 无锁系统统计数据缓存
pub struct SystemStatsCache {
    current_stats: AtomicPtr<SystemStats>,
    last_update: AtomicU64,
    ttl: Duration,
}

impl SystemStatsCache {
    /// 创建新的缓存实例
    #[inline]
    pub fn new(ttl: Duration) -> Self {
        Self {
            current_stats: AtomicPtr::new(Box::into_raw(Box::new(SystemStats::default()))),
            last_update: AtomicU64::new(0),
            ttl,
        }
    }

    /// 无锁读取缓存数据
    pub fn get(&self) -> Option<SystemStats> {
        // 先加载时间戳，避免 ABA 问题
        let last_update = self.last_update.load(Ordering::Acquire);
        if last_update == 0 {
            return None; // 未初始化
        }

        // 获取当前时间戳（使用毫秒精度）
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // 检查数据是否过期（使用毫秒精度）
        if now - last_update > self.ttl.as_millis() as u64 {
            return None; // 数据过期
        }

        // 加载数据指针
        let ptr = self.current_stats.load(Ordering::Acquire);
        if ptr.is_null() {
            return None;
        }

        // 安全读取数据
        let stats = unsafe { &*ptr };
        Some(stats.clone())
    }

    /// 原子更新缓存数据
    pub fn update(&self, new_stats: SystemStats) {
        // 先获取当前时间戳（使用毫秒精度）
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // 创建新数据
        let boxed_stats = Box::into_raw(Box::new(new_stats));

        // 原子替换数据指针
        let old_ptr = self.current_stats.swap(boxed_stats, Ordering::Release);

        // 安全释放旧数据
        if !old_ptr.is_null() {
            let _ = unsafe { Box::from_raw(old_ptr) };
        }

        // 最后更新时间戳，确保数据先于时间戳可见
        self.last_update.store(now, Ordering::Release);
    }

    /// 按需更新策略：只有在数据过期且有请求时才更新
    pub async fn get_or_update(&self) -> Result<SystemStats> {
        // 先尝试获取缓存
        if let Some(stats) = self.get() {
            return Ok(stats);
        }

        // 数据过期或不存在，重新获取
        let new_stats = collect_system_stats().await?;

        // 更新缓存
        self.update(new_stats.clone());
        Ok(new_stats)
    }
}

impl Drop for SystemStatsCache {
    fn drop(&mut self) {
        let ptr = self.current_stats.load(Ordering::Acquire);
        if !ptr.is_null() {
            let _ = unsafe { Box::from_raw(ptr) };
        }
    }
}

/// 缓存类型别名
pub type CacheRef = Arc<SystemStatsCache>;

/// 创建缓存实例的便捷函数
#[inline]
pub fn create_cache(ttl_seconds: u64) -> CacheRef {
    Arc::new(SystemStatsCache::new(Duration::from_secs(ttl_seconds)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;

    fn create_test_stats(hostname: &str, cpu_usage: f32) -> SystemStats {
        SystemStats {
            hostname: hostname.to_string(),
            cpu_usage,
            cpu_stats: crate::stats::CpuStats {
                overall: crate::stats::CpuUsageBreakdown {
                    user_percent: cpu_usage * 50.0,
                    nice_percent: cpu_usage * 10.0,
                    system_percent: cpu_usage * 40.0,
                    total_percent: cpu_usage * 100.0,
                },
                per_core: Vec::new(),
                core_count: 0,
            },
            memory_total: 1024 * 1024 * 1024,    // 1GB
            memory_used: 512 * 1024 * 1024,      // 512MB
            memory_available: 256 * 1024 * 1024, // 256MB
            memory_cached: 128 * 1024 * 1024,    // 128MB
            memory_free: 128 * 1024 * 1024,      // 128MB
            timestamp: std::time::Instant::now(),
        }
    }

    #[tokio::test]
    async fn test_cache_basic_operations() {
        let cache = SystemStatsCache::new(Duration::from_secs(10));

        // 初始状态应该返回 None
        assert!(cache.get().is_none());

        // 更新数据
        let stats = create_test_stats("test", 0.5);
        cache.update(stats.clone());

        // 应该能获取到数据
        let cached = cache.get().unwrap();
        assert_eq!(cached.hostname, "test");
        assert_eq!(cached.cpu_usage, 0.5);
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let cache = SystemStatsCache::new(Duration::from_millis(50)); // 减少过期时间

        let stats = create_test_stats("test", 0.5);
        cache.update(stats);

        // 立即获取应该成功
        assert!(cache.get().is_some());

        // 等待过期
        sleep(Duration::from_millis(100)).await;

        // 过期后应该返回 None
        assert!(cache.get().is_none());
    }

    #[tokio::test]
    async fn test_cache_creation() {
        let cache = SystemStatsCache::new(Duration::from_secs(10));

        // 初始状态应该返回 None
        assert!(cache.get().is_none());
    }

    #[tokio::test]
    async fn test_cache_update_and_get() {
        let cache = SystemStatsCache::new(Duration::from_secs(10));
        let stats = create_test_stats("test-host", 0.5);

        // 更新数据
        cache.update(stats.clone());

        // 应该能获取到数据
        let cached = cache.get().unwrap();
        assert_eq!(cached.hostname, "test-host");
        assert_eq!(cached.cpu_usage, 0.5);
        assert_eq!(cached.memory_total, 1024 * 1024 * 1024);
    }

    #[tokio::test]
    async fn test_cache_multiple_updates() {
        let cache = SystemStatsCache::new(Duration::from_secs(10));

        // 更新第一次
        let stats1 = create_test_stats("host1", 0.3);
        cache.update(stats1.clone());

        let cached1 = cache.get().unwrap();
        assert_eq!(cached1.hostname, "host1");
        assert_eq!(cached1.cpu_usage, 0.3);

        // 更新第二次
        let stats2 = create_test_stats("host2", 0.7);
        cache.update(stats2.clone());

        let cached2 = cache.get().unwrap();
        assert_eq!(cached2.hostname, "host2");
        assert_eq!(cached2.cpu_usage, 0.7);
    }

    #[tokio::test]
    async fn test_cache_drop() {
        let cache = SystemStatsCache::new(Duration::from_secs(10));
        let stats = create_test_stats("test-host", 0.5);
        cache.update(stats);

        // 验证数据存在
        assert!(cache.get().is_some());

        // Drop 缓存（测试 Drop 实现）
        drop(cache);

        // 如果没有 panic，说明 Drop 实现正确
    }

    #[tokio::test]
    async fn test_create_cache_function() {
        let cache = create_cache(5);

        // 验证缓存创建成功
        assert!(cache.get().is_none());

        // 测试更新和获取
        let stats = create_test_stats("test", 0.8);
        cache.update(stats.clone());

        let cached = cache.get().unwrap();
        assert_eq!(cached.hostname, "test");
        assert_eq!(cached.cpu_usage, 0.8);
    }

    #[tokio::test]
    async fn test_cache_concurrent_access() {
        let cache = Arc::new(SystemStatsCache::new(Duration::from_secs(10)));
        let stats = create_test_stats("concurrent-test", 0.6);
        cache.update(stats.clone());

        // 创建多个并发读取任务
        let mut handles = vec![];
        for i in 0..10 {
            let cache_clone = cache.clone();
            let handle = tokio::spawn(async move {
                let cached = cache_clone.get().unwrap();
                assert_eq!(cached.hostname, "concurrent-test");
                assert_eq!(cached.cpu_usage, 0.6);
                format!("task-{}-{}", i, cached.hostname)
            });
            handles.push(handle);
        }

        // 等待所有任务完成
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.starts_with("task-"));
            assert!(result.ends_with("concurrent-test"));
        }
    }

    #[tokio::test]
    async fn test_cache_zero_ttl() {
        let cache = SystemStatsCache::new(Duration::from_millis(1)); // 使用 1ms TTL
        let stats = create_test_stats("zero-ttl", 0.4);

        // 更新数据
        cache.update(stats);

        // 即使是零 TTL，也应该能立即获取到数据
        assert!(cache.get().is_some());

        // 等待一段时间后应该过期
        sleep(Duration::from_millis(10)).await;
        assert!(cache.get().is_none());
    }

    #[tokio::test]
    async fn test_cache_large_ttl() {
        let cache = SystemStatsCache::new(Duration::from_secs(3600)); // 1 小时
        let stats = create_test_stats("large-ttl", 0.9);

        // 更新数据
        cache.update(stats);

        // 应该能获取到数据
        assert!(cache.get().is_some());

        // 即使等待一段时间，也不应该过期
        sleep(Duration::from_millis(100)).await;
        assert!(cache.get().is_some());
    }
}
