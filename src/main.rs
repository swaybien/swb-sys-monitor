mod cache;
mod server;
mod stats;

use anyhow::Result;
use cache::create_cache;
use clap::Parser;
use log::info;
use server::{Config, StatusServer};

/// 资源占用显示系统
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// 服务器绑定地址 (默认: ::，支持 IPv4 和 IPv6)
    #[arg(short, long, default_value = "::")]
    address: String,

    /// 服务器端口 (默认: 8080)
    #[arg(short, long, default_value_t = 8080)]
    port: u16,

    /// 缓存 TTL 秒数 (默认: 10)
    #[arg(short, long, default_value_t = 10)]
    ttl: u64,

    /// 日志级别 (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // 初始化日志系统
    init_logger(&args.log_level);

    info!("资源占用显示系统启动中...");

    // 从命令行参数创建配置
    let config = Config {
        bind_address: args.address.clone(),
        port: args.port,
        cache_ttl_seconds: args.ttl,
    };

    info!(
        "配置信息 - 地址: {}, 端口: {}, 缓存 TTL: {} 秒",
        config.bind_address, config.port, config.cache_ttl_seconds
    );

    // 创建缓存
    let cache = create_cache(config.cache_ttl_seconds);
    info!("缓存系统初始化完成");

    // 创建服务器
    let server = StatusServer::new_with_ttl(cache, config.cache_ttl_seconds);
    info!("服务器实例创建完成");

    // 启动服务器
    let addr = config.address();
    info!("服务器将在 {addr} 启动");

    server.run(addr).await?;

    info!("服务器正常关闭");

    Ok(())
}

/// 初始化日志系统
fn init_logger(level: &str) {
    use std::env;

    // 设置默认日志格式
    unsafe {
        env::set_var("RUST_LOG", level);
    }

    // 初始化 env_logger
    match env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(level))
        .format_timestamp_secs()
        .try_init()
    {
        Ok(_) => info!("日志系统初始化成功，级别: {level}"),
        Err(e) => {
            eprintln!("日志系统初始化失败: {e}，使用默认设置");
            // 设置基本的日志输出
            env_logger::init();
        }
    }
}
