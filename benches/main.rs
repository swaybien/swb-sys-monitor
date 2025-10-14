use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::sync::Arc;
use std::time::Duration;
use swb_sys_monitor::cache::{SystemStatsCache, create_cache};
use swb_sys_monitor::server::StatusServer;
use swb_sys_monitor::stats::{CpuStats, CpuUsageBreakdown, SystemStats, collect_system_stats};
use tokio::runtime::Runtime;

fn create_test_stats(hostname: &str, cpu_usage: f32) -> SystemStats {
    SystemStats {
        hostname: hostname.to_string(),
        cpu_usage,
        cpu_stats: CpuStats {
            overall: CpuUsageBreakdown {
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

fn bench_cache_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("cache_creation", |b| {
        b.iter(|| {
            let cache = create_cache(black_box(10));
            black_box(cache);
        })
    });

    c.bench_function("cache_update_and_get", |b| {
        let cache = create_cache(10);
        let stats = create_test_stats("benchmark", 0.5);

        b.iter(|| {
            cache.update(black_box(stats.clone()));
            let result = cache.get();
            black_box(result);
        })
    });

    c.bench_function("cache_concurrent_reads", |b| {
        let cache = Arc::new(SystemStatsCache::new(Duration::from_secs(10)));
        let stats = create_test_stats("concurrent", 0.7);
        cache.update(stats);

        b.iter(|| {
            rt.block_on(async {
                let mut handles = vec![];
                for _ in 0..100 {
                    let cache_clone = cache.clone();
                    let handle = tokio::spawn(async move {
                        let result = cache_clone.get();
                        black_box(result);
                    });
                    handles.push(handle);
                }

                for handle in handles {
                    handle.await.unwrap();
                }
            })
        })
    });
}

fn bench_html_rendering(c: &mut Criterion) {
    let _server = StatusServer::new_with_ttl(create_cache(10), 10);
    let stats = create_test_stats("渲染测试主机", 0.65);

    c.bench_function("html_template_rendering", |b| {
        b.iter(|| {
            let html = StatusServer::render_html_template(black_box(&stats), 10);
            black_box(html);
        })
    });

    c.bench_function("html_rendering_with_large_values", |b| {
        let large_stats = SystemStats {
            hostname: "大型测试主机名称很长很长".to_string(),
            cpu_usage: 0.95,
            cpu_stats: CpuStats {
                overall: CpuUsageBreakdown {
                    user_percent: 47.5,
                    nice_percent: 9.5,
                    system_percent: 38.0,
                    total_percent: 95.0,
                },
                per_core: Vec::new(),
                core_count: 0,
            },
            memory_total: 16 * 1024 * 1024 * 1024,    // 16GB
            memory_used: 8 * 1024 * 1024 * 1024,      // 8GB
            memory_available: 4 * 1024 * 1024 * 1024, // 4GB
            memory_cached: 2 * 1024 * 1024 * 1024,    // 2GB
            memory_free: 2 * 1024 * 1024 * 1024,      // 2GB
            timestamp: std::time::Instant::now(),
        };

        b.iter(|| {
            let html = StatusServer::render_html_template(black_box(&large_stats), 10);
            black_box(html);
        })
    });
}

fn bench_system_stats_collection(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("system_stats_collection", |b| {
        b.iter(|| {
            rt.block_on(async {
                let stats = collect_system_stats().await;
                let _ = black_box(stats);
            })
        })
    });
}

fn bench_memory_allocation(c: &mut Criterion) {
    c.bench_function("string_allocation_with_capacity", |b| {
        b.iter(|| {
            let mut s = String::with_capacity(1024);
            s.push_str("测试字符串内容");
            black_box(s);
        })
    });

    c.bench_function("string_allocation_without_capacity", |b| {
        b.iter(|| {
            let mut s = String::new();
            s.push_str("测试字符串内容");
            black_box(s);
        })
    });
}

fn bench_cache_ttl_precision(c: &mut Criterion) {
    let cache = SystemStatsCache::new(Duration::from_millis(1));
    let stats = create_test_stats("ttl_test", 0.3);

    c.bench_function("cache_ttl_check", |b| {
        b.iter(|| {
            cache.update(black_box(stats.clone()));
            let result = cache.get();
            black_box(result);
        })
    });
}

criterion_group!(
    benches,
    bench_cache_operations,
    bench_html_rendering,
    bench_system_stats_collection,
    bench_memory_allocation,
    bench_cache_ttl_precision
);
criterion_main!(benches);
