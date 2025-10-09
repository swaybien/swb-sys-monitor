//! 资源占用显示系统库
//!
//! 这个库提供了一个极简的资源占用显示系统，专为嵌入式设备监控场景设计。

pub mod cache;
pub mod server;
pub mod stats;

// 重新导出主要的公共类型
pub use cache::{SystemStatsCache, create_cache};
pub use server::{Config, StatusServer};
pub use stats::{SystemStats, collect_system_stats};
