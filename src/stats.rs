use std::time::Instant;

/// 单个 CPU 核心的时间统计
#[derive(Debug, Default, Clone)]
pub struct CpuTimes {
    pub user: u64,   // 用户态时间
    pub nice: u64,   // 低优先级进程时间
    pub system: u64, // 内核态时间
    pub idle: u64,   // 空闲时间
    #[allow(dead_code)] // 这些字段用于完整的 CPU 时间统计，为未来功能预留
    pub iowait: u64, // I/O 等待时间
    #[allow(dead_code)] // 这些字段用于完整的 CPU 时间统计，为未来功能预留
    pub irq: u64, // 硬中断时间
    #[allow(dead_code)] // 这些字段用于完整的 CPU 时间统计，为未来功能预留
    pub softirq: u64, // 软中断时间
    pub total: u64,  // 总时间
}

/// CPU 使用率分解
#[derive(Debug, Default, Clone)]
pub struct CpuUsageBreakdown {
    pub user_percent: f32,   // 用户态使用率百分比
    pub nice_percent: f32,   // 低优先级进程使用率百分比
    pub system_percent: f32, // 内核态使用率百分比
    pub total_percent: f32,  // 总使用率百分比
}

/// 多核 CPU 统计信息
#[derive(Debug, Clone)]
pub struct CpuStats {
    pub overall: CpuUsageBreakdown,       // 总体 CPU 使用率
    pub per_core: Vec<CpuUsageBreakdown>, // 每个 CPU 核心的使用率
    pub core_count: usize,                // CPU 核心数量
}

use std::sync::Mutex;
// 注意：AtomicU64 和 Ordering 导入暂时保留，为未来优化预留
// #[allow(dead_code)] use std::sync::atomic::{AtomicU64, Ordering};

/// 全局 CPU 时间缓存，用于增量计算
static CPU_PREV_OVERALL: Mutex<Option<CpuTimes>> = Mutex::new(None);
static CPU_PREV_PER_CORE: Mutex<Vec<CpuTimes>> = Mutex::new(Vec::new());
static CPU_TIMES_INIT: std::sync::Once = std::sync::Once::new();

/// 系统资源统计数据结构
#[derive(Debug, Clone)]
pub struct SystemStats {
    pub hostname: String,
    pub cpu_usage: f32,        // CPU 使用率 (0.0-1.0) - 保持向后兼容
    pub cpu_stats: CpuStats,   // 详细的 CPU 统计信息
    pub memory_total: u64,     // 总内存字节数
    pub memory_used: u64,      // 已用内存字节数
    pub memory_available: u64, // 可用内存字节数
    pub memory_cached: u64,    // 缓存内存字节数
    pub memory_free: u64,      // 空闲内存字节数
    pub timestamp: Instant,    // 数据获取时间戳
}

impl Default for SystemStats {
    #[inline]
    fn default() -> Self {
        Self {
            hostname: "未知主机".to_string(),
            cpu_usage: 0.0,
            cpu_stats: CpuStats {
                overall: CpuUsageBreakdown::default(),
                per_core: Vec::new(),
                core_count: 0,
            },
            memory_total: 0,
            memory_used: 0,
            memory_available: 0,
            memory_cached: 0,
            memory_free: 0,
            timestamp: Instant::now(),
        }
    }
}

/// 系统资源获取错误类型
#[derive(Debug)]
pub enum StatsError {
    IoError(std::io::Error),
    ParseError(String),
    #[allow(dead_code)] // 为未来跨平台支持预留
    UnsupportedPlatform,
}

impl From<std::io::Error> for StatsError {
    #[inline]
    fn from(error: std::io::Error) -> Self {
        StatsError::IoError(error)
    }
}

impl std::fmt::Display for StatsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StatsError::IoError(e) => write!(f, "IO 错误: {e}"),
            StatsError::ParseError(s) => write!(f, "解析错误: {s}"),
            StatsError::UnsupportedPlatform => write!(f, "不支持的平台"),
        }
    }
}

impl std::error::Error for StatsError {}

pub type Result<T> = std::result::Result<T, StatsError>;

/// 收集系统统计数据
pub async fn collect_system_stats() -> Result<SystemStats> {
    #[cfg(target_os = "linux")]
    {
        collect_linux_stats().await
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err(StatsError::UnsupportedPlatform)
    }
}

/// Linux 系统统计数据收集
#[cfg(target_os = "linux")]
async fn collect_linux_stats() -> Result<SystemStats> {
    let hostname = get_hostname()?;
    let cpu_stats = get_cpu_stats().await?;
    let cpu_usage = cpu_stats.overall.total_percent / 100.0; // 转换为 0.0-1.0 范围
    let memory_info = get_memory_info().await?;

    Ok(SystemStats {
        hostname,
        cpu_usage,
        cpu_stats,
        memory_total: memory_info.total,
        memory_used: memory_info.used,
        memory_available: memory_info.available,
        memory_cached: memory_info.cached,
        memory_free: memory_info.free,
        timestamp: Instant::now(),
    })
}

/// 获取主机名
#[cfg(target_os = "linux")]
#[inline]
fn get_hostname() -> Result<String> {
    std::fs::read_to_string("/proc/sys/kernel/hostname")
        .map(|s| s.trim().to_string())
        .map_err(From::from)
}

/// 内存信息结构
#[derive(Debug, Default)]
struct MemoryInfo {
    total: u64,
    used: u64,
    available: u64,
    cached: u64,
    free: u64,
}

/// 获取内存信息
#[cfg(target_os = "linux")]
async fn get_memory_info() -> Result<MemoryInfo> {
    // 预估 /proc/meminfo 的大小，预分配容量
    let mut content = String::with_capacity(2048);
    let file_content = tokio::fs::read_to_string("/proc/meminfo").await?;
    content.push_str(&file_content);

    let mut info = MemoryInfo::default();

    for line in content.lines() {
        let mut parts = line.split_whitespace();
        if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
            let value = value.parse::<u64>().unwrap_or(0) * 1024; // 转换为字节

            match key {
                "MemTotal:" => info.total = value,
                "MemAvailable:" => info.available = value,
                "Cached:" => info.cached = value,
                "MemFree:" => info.free = value,
                _ => {}
            }
        }
    }

    // 计算已用内存 = 总内存 - 可用内存
    info.used = info.total.saturating_sub(info.available);

    Ok(info)
}

/// 解析 CPU 时间统计（为未来功能预留）
#[cfg(target_os = "linux")]
#[inline]
#[allow(dead_code)] // 为未来功能预留
fn parse_cpu_times(content: &str) -> Result<CpuTimes> {
    // 解析第一行 CPU 总时间
    let first_line = content
        .lines()
        .next()
        .ok_or_else(|| StatsError::ParseError("无法解析 /proc/stat".to_string()))?;

    let mut parts = first_line.split_whitespace().skip(1); // 跳过 "cpu"

    let user: u64 = parts.next().unwrap_or("0").parse().unwrap_or(0);
    let nice: u64 = parts.next().unwrap_or("0").parse().unwrap_or(0);
    let system: u64 = parts.next().unwrap_or("0").parse().unwrap_or(0);
    let idle: u64 = parts.next().unwrap_or("0").parse().unwrap_or(0);
    let iowait: u64 = parts.next().unwrap_or("0").parse().unwrap_or(0);
    let irq: u64 = parts.next().unwrap_or("0").parse().unwrap_or(0);
    let softirq: u64 = parts.next().unwrap_or("0").parse().unwrap_or(0);

    // 忽略其他字段 (steal, guest, guest_nice)
    let total = user + nice + system + idle + iowait + irq + softirq;

    Ok(CpuTimes {
        user,
        nice,
        system,
        idle,
        iowait,
        irq,
        softirq,
        total,
    })
}

/// 解析所有 CPU 核心的时间统计
#[cfg(target_os = "linux")]
#[inline]
fn parse_all_cpu_times(content: &str) -> Result<(CpuTimes, Vec<CpuTimes>)> {
    let lines = content.lines();
    let mut overall_times = None;
    let mut per_core_times = Vec::new();

    for line in lines {
        if line.starts_with("cpu") {
            let mut parts = line.split_whitespace();
            let cpu_label = parts.next().unwrap_or("");

            let user: u64 = parts.next().unwrap_or("0").parse().unwrap_or(0);
            let nice: u64 = parts.next().unwrap_or("0").parse().unwrap_or(0);
            let system: u64 = parts.next().unwrap_or("0").parse().unwrap_or(0);
            let idle: u64 = parts.next().unwrap_or("0").parse().unwrap_or(0);
            let iowait: u64 = parts.next().unwrap_or("0").parse().unwrap_or(0);
            let irq: u64 = parts.next().unwrap_or("0").parse().unwrap_or(0);
            let softirq: u64 = parts.next().unwrap_or("0").parse().unwrap_or(0);

            let total = user + nice + system + idle + iowait + irq + softirq;
            let times = CpuTimes {
                user,
                nice,
                system,
                idle,
                iowait,
                irq,
                softirq,
                total,
            };

            if cpu_label == "cpu" {
                overall_times = Some(times);
            } else if cpu_label.starts_with("cpu") {
                per_core_times.push(times);
            }
        }
    }

    match overall_times {
        Some(overall) => Ok((overall, per_core_times)),
        None => Err(StatsError::ParseError("无法找到 CPU 统计信息".to_string())),
    }
}

/// CPU 使用率计算（使用增量算法）（为向后兼容预留）
#[cfg(target_os = "linux")]
#[allow(dead_code)] // 为向后兼容预留
async fn get_cpu_usage() -> Result<f32> {
    let cpu_stats = get_cpu_stats().await?;
    Ok(cpu_stats.overall.total_percent / 100.0)
}

/// 获取详细的 CPU 统计信息
#[cfg(target_os = "linux")]
async fn get_cpu_stats() -> Result<CpuStats> {
    // 预估 /proc/stat 的大小，预分配容量
    let mut content = String::with_capacity(2048);
    let file_content = tokio::fs::read_to_string("/proc/stat").await?;
    content.push_str(&file_content);

    let (current_overall, current_per_core) = parse_all_cpu_times(&content)?;

    // 获取之前的时间统计（线程安全）
    let (prev_overall, prev_per_core) = {
        let mut prev_overall_guard = CPU_PREV_OVERALL.lock().unwrap();
        let mut prev_per_core_guard = CPU_PREV_PER_CORE.lock().unwrap();

        CPU_TIMES_INIT.call_once(|| {
            *prev_overall_guard = Some(current_overall.clone());
            prev_per_core_guard.clone_from(&current_per_core);
        });

        (prev_overall_guard.clone(), prev_per_core_guard.clone())
    };

    // 如果是第一次调用，返回 0 使用率
    let overall_usage = if let (Some(prev_overall), _) = (&prev_overall, &prev_per_core) {
        calculate_cpu_usage_breakdown(prev_overall, &current_overall)
    } else {
        CpuUsageBreakdown::default()
    };

    // 计算每个 CPU 核心的使用率
    let mut per_core_usage = Vec::new();
    for (i, current_core) in current_per_core.iter().enumerate() {
        if let Some(prev_core) = prev_per_core.get(i) {
            per_core_usage.push(calculate_cpu_usage_breakdown(prev_core, current_core));
        } else {
            per_core_usage.push(CpuUsageBreakdown::default());
        }
    }

    // 更新全局缓存
    {
        let mut prev_overall_guard = CPU_PREV_OVERALL.lock().unwrap();
        let mut prev_per_core_guard = CPU_PREV_PER_CORE.lock().unwrap();
        *prev_overall_guard = Some(current_overall.clone());
        *prev_per_core_guard = current_per_core.clone();
    }

    Ok(CpuStats {
        overall: overall_usage,
        per_core: per_core_usage,
        core_count: current_per_core.len(),
    })
}

/// 计算两个时间点之间的 CPU 使用率分解
#[inline]
fn calculate_cpu_usage_breakdown(prev: &CpuTimes, current: &CpuTimes) -> CpuUsageBreakdown {
    // 计算增量
    let total_diff = current.total.saturating_sub(prev.total);

    if total_diff == 0 {
        return CpuUsageBreakdown::default();
    }

    let user_diff = current.user.saturating_sub(prev.user);
    let nice_diff = current.nice.saturating_sub(prev.nice);
    let system_diff = current.system.saturating_sub(prev.system);
    let idle_diff = current.idle.saturating_sub(prev.idle);

    // 计算各分量的使用率百分比
    let user_percent = (user_diff as f32 / total_diff as f32) * 100.0;
    let nice_percent = (nice_diff as f32 / total_diff as f32) * 100.0;
    let system_percent = (system_diff as f32 / total_diff as f32) * 100.0;
    let total_percent = (total_diff.saturating_sub(idle_diff) as f32 / total_diff as f32) * 100.0;

    CpuUsageBreakdown {
        user_percent: user_percent.clamp(0.0, 100.0),
        nice_percent: nice_percent.clamp(0.0, 100.0),
        system_percent: system_percent.clamp(0.0, 100.0),
        total_percent: total_percent.clamp(0.0, 100.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_stats_default() {
        let stats = SystemStats::default();
        assert_eq!(stats.hostname, "未知主机");
        assert_eq!(stats.cpu_usage, 0.0);
        assert_eq!(stats.memory_total, 0);
        assert_eq!(stats.memory_used, 0);
        assert_eq!(stats.memory_available, 0);
        assert_eq!(stats.memory_cached, 0);
        assert_eq!(stats.memory_free, 0);
    }

    #[test]
    fn test_cpu_times_default() {
        let times = CpuTimes::default();
        assert_eq!(times.user, 0);
        assert_eq!(times.nice, 0);
        assert_eq!(times.system, 0);
        assert_eq!(times.idle, 0);
        assert_eq!(times.total, 0);
    }

    #[test]
    fn test_parse_cpu_times_valid() {
        let content = "cpu  1234 567 890 1234 0 0 0 0 0 0";
        let times = parse_cpu_times(content).unwrap();
        assert_eq!(times.user, 1234);
        assert_eq!(times.nice, 567);
        assert_eq!(times.system, 890);
        assert_eq!(times.idle, 1234);
        assert_eq!(times.total, 1234 + 567 + 890 + 1234);
    }

    #[test]
    fn test_parse_cpu_times_invalid() {
        let content = "invalid content";
        let result = parse_cpu_times(content).unwrap();
        assert_eq!(result.total, 0);
        assert_eq!(result.idle, 0);
    }

    #[test]
    fn test_parse_cpu_times_empty() {
        let content = "";
        assert!(parse_cpu_times(content).is_err());
    }

    #[test]
    fn test_stats_error_display() {
        let io_error = StatsError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "test error",
        ));
        assert_eq!(format!("{}", io_error), "IO 错误: test error");

        let parse_error = StatsError::ParseError("test parse error".to_string());
        assert_eq!(format!("{}", parse_error), "解析错误: test parse error");

        let unsupported_error = StatsError::UnsupportedPlatform;
        assert_eq!(format!("{}", unsupported_error), "不支持的平台");
    }

    #[test]
    fn test_stats_error_from_io() {
        let io_error =
            std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission denied");
        let stats_error = StatsError::from(io_error);
        match stats_error {
            StatsError::IoError(_) => {} // 预期的类型
            _ => panic!("应该是 IoError 类型"),
        }
    }

    #[tokio::test]
    #[cfg(target_os = "linux")]
    async fn test_get_hostname() {
        // 测试获取主机名
        match get_hostname() {
            Ok(hostname) => {
                assert!(!hostname.is_empty());
                println!("主机名: {}", hostname);
            }
            Err(e) => {
                // 在某些环境中可能失败，这是可以接受的
                println!("获取主机名失败: {}", e);
            }
        }
    }

    #[tokio::test]
    #[cfg(target_os = "linux")]
    async fn test_get_memory_info() {
        // 测试获取内存信息
        match get_memory_info().await {
            Ok(info) => {
                assert!(info.total > 0);
                assert!(info.used <= info.total);
                assert!(info.available <= info.total);
                assert!(info.cached <= info.total);
                assert!(info.free <= info.total);
                println!("内存信息: {:?}", info);
            }
            Err(e) => {
                // 在某些环境中可能失败
                println!("获取内存信息失败: {}", e);
            }
        }
    }

    #[tokio::test]
    #[cfg(target_os = "linux")]
    async fn test_collect_linux_stats() {
        // 测试完整的 Linux 统计数据收集
        match collect_linux_stats().await {
            Ok(stats) => {
                assert!(!stats.hostname.is_empty());
                assert!(stats.cpu_usage >= 0.0 && stats.cpu_usage <= 1.0);
                assert!(stats.memory_total > 0);
                println!("系统统计: {:?}", stats);
            }
            Err(e) => {
                // 在某些环境中可能失败
                println!("收集系统统计失败: {}", e);
            }
        }
    }

    #[tokio::test]
    #[cfg(not(target_os = "linux"))]
    async fn test_collect_system_stats_unsupported() {
        // 测试非 Linux 平台
        let result = collect_system_stats().await;
        assert!(result.is_err());
        match result.unwrap_err() {
            StatsError::UnsupportedPlatform => {} // 预期的错误
            _ => panic!("应该是 UnsupportedPlatform 错误"),
        }
    }

    #[test]
    fn test_cpu_usage_breakdown_default() {
        let breakdown = CpuUsageBreakdown::default();
        assert_eq!(breakdown.user_percent, 0.0);
        assert_eq!(breakdown.nice_percent, 0.0);
        assert_eq!(breakdown.system_percent, 0.0);
        assert_eq!(breakdown.total_percent, 0.0);
    }

    #[test]
    fn test_parse_all_cpu_times_valid() {
        let content = "cpu  1234 567 890 1234 100 200 300 0 0 0\n\
                        cpu0 617 283 445 617 50 100 150\n\
                        cpu1 617 284 445 617 50 100 150";
        let (overall, per_core) = parse_all_cpu_times(content).unwrap();

        assert_eq!(overall.user, 1234);
        assert_eq!(overall.nice, 567);
        assert_eq!(overall.system, 890);
        assert_eq!(overall.idle, 1234);
        assert_eq!(overall.iowait, 100);
        assert_eq!(overall.irq, 200);
        assert_eq!(overall.softirq, 300);

        assert_eq!(per_core.len(), 2);
        assert_eq!(per_core[0].user, 617);
        assert_eq!(per_core[1].user, 617);
    }

    #[test]
    fn test_calculate_cpu_usage_breakdown() {
        let prev = CpuTimes {
            user: 100,
            nice: 20,
            system: 50,
            idle: 800,
            iowait: 10,
            irq: 5,
            softirq: 15,
            total: 1000,
        };

        let current = CpuTimes {
            user: 200,
            nice: 30,
            system: 80,
            idle: 1500,
            iowait: 20,
            irq: 10,
            softirq: 20,
            total: 1860,
        };

        let breakdown = calculate_cpu_usage_breakdown(&prev, &current);

        // 计算增量：total_diff = 860, user_diff = 100, nice_diff = 10, system_diff = 30, idle_diff = 700
        assert!((breakdown.user_percent - 11.63).abs() < 0.1); // 100/860 * 100
        assert!((breakdown.nice_percent - 1.16).abs() < 0.1); // 10/860 * 100
        assert!((breakdown.system_percent - 3.49).abs() < 0.1); // 30/860 * 100
        assert!((breakdown.total_percent - 18.60).abs() < 0.1); // 160/860 * 100
    }

    #[test]
    fn test_calculate_cpu_usage_breakdown_zero_diff() {
        let prev = CpuTimes {
            user: 100,
            nice: 20,
            system: 50,
            idle: 800,
            iowait: 10,
            irq: 5,
            softirq: 15,
            total: 1000,
        };

        let current = prev.clone();
        let breakdown = calculate_cpu_usage_breakdown(&prev, &current);

        assert_eq!(breakdown.user_percent, 0.0);
        assert_eq!(breakdown.nice_percent, 0.0);
        assert_eq!(breakdown.system_percent, 0.0);
        assert_eq!(breakdown.total_percent, 0.0);
    }

    #[tokio::test]
    #[cfg(target_os = "linux")]
    async fn test_get_cpu_stats() {
        // 测试获取 CPU 统计信息
        match get_cpu_stats().await {
            Ok(stats) => {
                assert!(stats.core_count > 0);
                assert!(stats.per_core.len() == stats.core_count);
                assert!(stats.overall.total_percent >= 0.0 && stats.overall.total_percent <= 100.0);

                // 检查各个分量的合理性
                assert!(stats.overall.user_percent >= 0.0 && stats.overall.user_percent <= 100.0);
                assert!(stats.overall.nice_percent >= 0.0 && stats.overall.nice_percent <= 100.0);
                assert!(
                    stats.overall.system_percent >= 0.0 && stats.overall.system_percent <= 100.0
                );

                println!("CPU 统计: {:?}", stats);
            }
            Err(e) => {
                // 在某些环境中可能失败
                println!("获取 CPU 统计失败: {}", e);
            }
        }
    }

    #[test]
    fn test_cpu_times_new_fields() {
        // 更新现有的测试以包含新字段
        let times = CpuTimes::default();
        assert_eq!(times.user, 0);
        assert_eq!(times.nice, 0);
        assert_eq!(times.system, 0);
        assert_eq!(times.idle, 0);
        assert_eq!(times.iowait, 0); // 新字段
        assert_eq!(times.irq, 0); // 新字段
        assert_eq!(times.softirq, 0); // 新字段
        assert_eq!(times.total, 0);
    }
}
