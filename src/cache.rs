use crate::stats::{Result, SystemStats, collect_system_stats};
use arc_swap::ArcSwap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// 无锁系统统计数据缓存
pub struct SystemStatsCache {
    current_stats: ArcSwap<SystemStats>,
    last_update: AtomicU64,
    ttl: Duration,
}

impl SystemStatsCache {
    /// 创建新的缓存实例
    #[inline]
    pub fn new(ttl: Duration) -> Self {
        Self {
            current_stats: ArcSwap::from_pointee(SystemStats::default()),
            last_update: AtomicU64::new(0),
            ttl,
        }
    }

    /// 无锁读取缓存数据
    pub fn get(&self) -> Option<Arc<SystemStats>> {
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

        // 检查数据是否过期（使用毫秒精度，saturating_sub 防止时钟回拨导致下溢 panic）
        if now.saturating_sub(last_update) > self.ttl.as_millis() as u64 {
            return None; // 数据过期
        }

        // 无锁加载数据快照（ArcSwap 自动管理生命周期，无 use-after-free 风险）
        Some(self.current_stats.load_full())
    }

    /// 原子更新缓存数据
    pub fn update(&self, new_stats: SystemStats) {
        // 先获取当前时间戳（使用毫秒精度）
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // 原子替换数据（旧值由 Arc 引用计数自动释放，并发读者仍可安全持有）
        self.current_stats.store(Arc::new(new_stats));

        // 最后更新时间戳，确保数据先于时间戳可见
        self.last_update.store(now, Ordering::Release);
    }

    /// 按需更新策略：只有在数据过期且有请求时才更新
    pub async fn get_or_update(&self) -> Result<Arc<SystemStats>> {
        // 先尝试获取缓存
        if let Some(stats) = self.get() {
            return Ok(stats);
        }

        // 数据过期或不存在，重新获取
        let new_stats = collect_system_stats().await?;

        // 更新缓存
        self.update(new_stats.clone());
        Ok(Arc::new(new_stats))
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

        // Drop 缓存（ArcSwap 自动管理内存，无需手动释放）
        drop(cache);

        // 如果没有 panic，说明资源释放正确
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

    /// 并发冒烟测试：多线程同时读写不崩溃、不出现内存安全问题
    #[test]
    fn test_cache_concurrent_smoke() {
        let cache = Arc::new(SystemStatsCache::new(Duration::from_secs(10)));
        let mut handles = vec![];

        // 多个读者线程循环 get / get_or_update
        for _ in 0..4 {
            let cache = cache.clone();
            handles.push(std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("创建运行时失败");
                for _ in 0..200 {
                    if let Some(stats) = cache.get() {
                        assert!(!stats.hostname.is_empty() || stats.cpu_usage >= 0.0);
                    }
                    let _ = rt.block_on(cache.get_or_update());
                }
            }));
        }

        // 一个写者线程循环 update
        {
            let cache = cache.clone();
            handles.push(std::thread::spawn(move || {
                for i in 0..400 {
                    cache.update(create_test_stats("writer", i as f32 / 100.0));
                }
            }));
        }

        for handle in handles {
            handle.join().expect("线程不应 panic");
        }
    }
}
