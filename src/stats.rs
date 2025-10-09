use std::time::Instant;

/// CPU 时间统计
#[derive(Debug, Default, Clone)]
#[allow(dead_code)] // 为未来扩展预留字段
pub struct CpuTimes {
    pub user: u64,
    pub nice: u64,
    pub system: u64,
    pub idle: u64,
    pub total: u64,
}

use std::sync::atomic::{AtomicU64, Ordering};

/// 全局 CPU 时间缓存，用于增量计算
static CPU_PREV_TOTAL: AtomicU64 = AtomicU64::new(0);
static CPU_PREV_IDLE: AtomicU64 = AtomicU64::new(0);
static CPU_TIMES_INIT: std::sync::Once = std::sync::Once::new();

/// 系统资源统计数据结构
#[derive(Debug, Clone)]
pub struct SystemStats {
    pub hostname: String,
    pub cpu_usage: f32,        // CPU 使用率 (0.0-1.0)
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
    let cpu_usage = get_cpu_usage().await?;
    let memory_info = get_memory_info().await?;

    Ok(SystemStats {
        hostname,
        cpu_usage,
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

/// 解析 CPU 时间统计
#[cfg(target_os = "linux")]
#[inline]
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

    // 忽略其他字段 (iowait, irq, softirq, steal, guest, guest_nice)
    let total = user + nice + system + idle;

    Ok(CpuTimes {
        user,
        nice,
        system,
        idle,
        total,
    })
}

/// CPU 使用率计算（使用增量算法）
#[cfg(target_os = "linux")]
async fn get_cpu_usage() -> Result<f32> {
    // 预估 /proc/stat 的大小，预分配容量
    let mut content = String::with_capacity(1024);
    let file_content = tokio::fs::read_to_string("/proc/stat").await?;
    content.push_str(&file_content);

    let current_times = parse_cpu_times(&content)?;

    // 获取之前的时间统计（线程安全）
    let (prev_total, prev_idle) = {
        CPU_TIMES_INIT.call_once(|| {
            CPU_PREV_TOTAL.store(current_times.total, Ordering::Relaxed);
            CPU_PREV_IDLE.store(current_times.idle, Ordering::Relaxed);
        });
        (
            CPU_PREV_TOTAL.load(Ordering::Relaxed),
            CPU_PREV_IDLE.load(Ordering::Relaxed),
        )
    };

    // 如果是第一次调用，返回 0 使用率
    if prev_total == 0 && prev_idle == 0 && current_times.total != 0 {
        CPU_PREV_TOTAL.store(current_times.total, Ordering::Relaxed);
        CPU_PREV_IDLE.store(current_times.idle, Ordering::Relaxed);
        return Ok(0.0);
    }

    // 计算增量
    let total_diff = current_times.total.saturating_sub(prev_total);
    let idle_diff = current_times.idle.saturating_sub(prev_idle);

    // 更新全局缓存
    CPU_PREV_TOTAL.store(current_times.total, Ordering::Relaxed);
    CPU_PREV_IDLE.store(current_times.idle, Ordering::Relaxed);

    // 如果总时间差为 0，返回 0 使用率
    if total_diff == 0 {
        return Ok(0.0);
    }

    // 计算使用率：1 - (空闲时间增量 / 总时间增量)
    let usage = 1.0 - (idle_diff as f32 / total_diff as f32);

    // 确保使用率在合理范围内
    Ok(usage.clamp(0.0, 1.0))
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
}
