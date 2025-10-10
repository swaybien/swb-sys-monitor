use crate::cache::CacheRef;
use anyhow::Result;
use hyper::http::StatusCode;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server};
use log::{error, info, warn};
use std::convert::Infallible;
use std::net::SocketAddr;

/// 状态服务器
pub struct StatusServer {
    cache: CacheRef,
}

impl StatusServer {
    /// 创建新的状态服务器实例
    #[inline]
    pub fn new(cache: CacheRef) -> Self {
        Self { cache }
    }

    /// 运行服务器
    pub async fn run(self, addr: SocketAddr) -> Result<()> {
        let cache = self.cache;

        let make_svc = make_service_fn(move |_conn| {
            let cache = cache.clone();
            async move {
                Ok::<_, Infallible>(service_fn(move |req| {
                    let cache = cache.clone();
                    Self::handle_request(req, cache)
                }))
            }
        });

        // 创建服务器并配置高并发参数
        let server = Server::bind(&addr)
            .http1_keepalive(true)
            .http1_half_close(false)
            .tcp_keepalive(Some(std::time::Duration::from_secs(10)))
            .tcp_nodelay(true)
            .serve(make_svc);

        info!("服务器运行在: http://{addr}");
        info!("已启用高并发模式，支持 HTTP/1.1 keep-alive");

        server.await.map_err(|e| {
            error!("服务器错误: {e}");
            anyhow::anyhow!("服务器运行错误: {e}")
        })?;

        info!("服务器正常关闭");
        Ok(())
    }

    /// 处理 HTTP 请求
    async fn handle_request(
        req: Request<Body>,
        cache: CacheRef,
    ) -> std::result::Result<Response<Body>, Infallible> {
        // 添加连接信息头部，便于调试
        match (req.method(), req.uri().path()) {
            (&Method::GET, "/") => {
                match Self::serve_html(cache).await {
                    Ok(mut response) => {
                        // 添加缓存控制头，允许客户端在 10 秒内使用缓存
                        // 与 HTML meta refresh 和服务器缓存 TTL 保持一致，减少服务器负载
                        response.headers_mut().insert(
                            "Cache-Control",
                            hyper::header::HeaderValue::from_static("public, max-age=10"),
                        );
                        Ok(response)
                    }
                    Err(_) => Ok(Self::serve_error(
                        "数据获取失败".to_string(),
                        StatusCode::INTERNAL_SERVER_ERROR,
                    )),
                }
            }
            (&Method::GET, "/health") => Ok(Self::serve_health()),
            _ => Ok(Self::serve_404()),
        }
    }

    /// 提供健康检查端点
    #[inline]
    fn serve_health() -> Response<Body> {
        Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/plain")
            .header("Cache-Control", "no-cache")
            .body(Body::from("OK"))
            .unwrap()
    }

    /// 提供 404 页面
    #[inline]
    fn serve_404() -> Response<Body> {
        warn!("请求了不存在的页面");
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header("content-type", "text/plain; charset=utf-8")
            .body(Body::from("页面未找到"))
            .unwrap()
    }

    /// 提供错误页面
    #[inline]
    fn serve_error(message: String, status: StatusCode) -> Response<Body> {
        Response::builder()
            .status(status)
            .header("content-type", "text/plain; charset=utf-8")
            .body(Body::from(message))
            .unwrap()
    }

    /// 提供主页面
    async fn serve_html(cache: CacheRef) -> Result<Response<Body>> {
        // 获取系统数据
        let stats = cache.get_or_update().await.map_err(|e| {
            error!("获取系统数据失败: {e}");
            e
        })?;

        // 渲染 HTML 模板
        let html = Self::render_html_template(&stats);

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/html; charset=utf-8")
            .body(Body::from(html))
            .unwrap())
    }

    /// 渲染 HTML 模板
    pub fn render_html_template(stats: &crate::stats::SystemStats) -> String {
        let total_mb = stats.memory_total / 1024 / 1024;
        let used_mb = stats.memory_used / 1024 / 1024;
        let available_mb = stats.memory_available / 1024 / 1024;
        let cached_mb = stats.memory_cached / 1024 / 1024;
        let free_mb = stats.memory_free / 1024 / 1024;

        let cpu_percent = (stats.cpu_usage * 100.0) as u32;
        let cpu_user_percent = stats.cpu_stats.overall.user_percent as u32;
        let cpu_system_percent = stats.cpu_stats.overall.system_percent as u32;
        let cpu_nice_percent = stats.cpu_stats.overall.nice_percent as u32;

        // 生成多核 CPU 部分
        let cpu_cores_section = if stats.cpu_stats.core_count > 0 {
            let mut cores_html = String::from("<fieldset><legend>处理器 - 各核心使用率</legend>");
            for (i, core_stats) in stats.cpu_stats.per_core.iter().enumerate() {
                cores_html.push_str(&format!(
                    "<p>核心 {}：<progress value=\"{}\" max=\"100\">{}%</progress></p>",
                    i, core_stats.total_percent as u32, core_stats.total_percent as u32
                ));
            }
            cores_html.push_str("</fieldset>");
            cores_html
        } else {
            String::new()
        };

        // 格式化时间戳为可读格式
        let timestamp = format!("{:?}", stats.timestamp);

        // 使用内置模板（编译进二进制文件）
        let template = include_str!("../templates/index.html");

        // 使用 String::with_capacity 预分配容量，减少重新分配
        let mut result = String::with_capacity(template.len() + 512);

        // 手动替换变量，避免多次字符串分配
        result.push_str(template);
        result = result.replace("{hostname}", &stats.hostname);
        result = result.replace("{cpu_percent}", &cpu_percent.to_string());
        result = result.replace("{cpu_user_percent}", &cpu_user_percent.to_string());
        result = result.replace("{cpu_system_percent}", &cpu_system_percent.to_string());
        result = result.replace("{cpu_nice_percent}", &cpu_nice_percent.to_string());
        result = result.replace("{cpu_cores_section}", &cpu_cores_section);
        result = result.replace("{memory_total_mb}", &total_mb.to_string());
        result = result.replace("{memory_used_mb}", &used_mb.to_string());
        result = result.replace("{memory_available_mb}", &available_mb.to_string());
        result = result.replace("{memory_cached_mb}", &cached_mb.to_string());
        result = result.replace("{memory_free_mb}", &free_mb.to_string());
        result = result.replace("{timestamp}", &timestamp);

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::create_cache;
    use crate::stats::SystemStats;
    use hyper::{Body, Request, StatusCode};
    use std::time::Instant;

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
            timestamp: Instant::now(),
        }
    }

    #[tokio::test]
    async fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.bind_address, "0.0.0.0");
        assert_eq!(config.port, 8080);
        assert_eq!(config.cache_ttl_seconds, 10);
    }

    #[tokio::test]
    async fn test_config_address() {
        let config = Config::default();
        let addr = config.address();
        assert_eq!(addr.to_string(), "0.0.0.0:8080");
    }

    #[tokio::test]
    async fn test_status_server_creation() {
        let cache = create_cache(10);
        let _server = StatusServer::new(cache);
        // 服务器创建成功，没有 panic
    }

    #[tokio::test]
    async fn test_serve_health() {
        let response = StatusServer::serve_health();
        assert_eq!(response.status(), StatusCode::OK);

        let headers = response.headers();
        assert_eq!(headers.get("content-type").unwrap(), "text/plain");

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        assert_eq!(std::str::from_utf8(&body).unwrap(), "OK");
    }

    #[tokio::test]
    async fn test_serve_404() {
        let response = StatusServer::serve_404();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let headers = response.headers();
        assert_eq!(
            headers.get("content-type").unwrap(),
            "text/plain; charset=utf-8"
        );

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        assert_eq!(std::str::from_utf8(&body).unwrap(), "页面未找到");
    }

    #[tokio::test]
    async fn test_serve_error() {
        let message = "测试错误".to_string();
        let response =
            StatusServer::serve_error(message.clone(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let headers = response.headers();
        assert_eq!(
            headers.get("content-type").unwrap(),
            "text/plain; charset=utf-8"
        );

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        assert_eq!(std::str::from_utf8(&body).unwrap(), message);
    }

    #[tokio::test]
    async fn test_render_html_template() {
        let stats = create_test_stats("测试主机", 0.75);
        let html = StatusServer::render_html_template(&stats);

        // 检查 HTML 是否包含预期的内容
        assert!(html.contains("测试主机"));
        assert!(html.contains("75"));
        assert!(html.contains("1024")); // 内存总量 MB
        assert!(html.contains("512")); // 已用内存 MB
        assert!(html.contains("256")); // 可用内存 MB
        assert!(html.contains("128")); // 缓存内存 MB

        // 检查 CPU 详细分解
        assert!(html.contains("处理器"));
        assert!(html.contains("用户态"));
        assert!(html.contains("内核态"));
        assert!(html.contains("低优先级"));
    }

    #[tokio::test]
    async fn test_render_html_template_special_chars() {
        let stats = create_test_stats("主机<>&\"'", 0.5);
        let html = StatusServer::render_html_template(&stats);

        // 检查特殊字符是否被正确处理
        assert!(html.contains("主机<>&\"'"));
        assert!(html.contains("50"));
    }

    #[tokio::test]
    async fn test_render_html_template_memory_values() {
        let stats = SystemStats {
            hostname: "test".to_string(),
            cpu_usage: 0.5,
            cpu_stats: crate::stats::CpuStats {
                overall: crate::stats::CpuUsageBreakdown {
                    user_percent: 25.0,
                    nice_percent: 5.0,
                    system_percent: 20.0,
                    total_percent: 50.0,
                },
                per_core: vec![
                    crate::stats::CpuUsageBreakdown {
                        user_percent: 30.0,
                        nice_percent: 5.0,
                        system_percent: 15.0,
                        total_percent: 50.0,
                    },
                    crate::stats::CpuUsageBreakdown {
                        user_percent: 20.0,
                        nice_percent: 5.0,
                        system_percent: 25.0,
                        total_percent: 50.0,
                    },
                ],
                core_count: 2,
            },
            memory_total: 2048 * 1024 * 1024,    // 2GB
            memory_used: 1024 * 1024 * 1024,     // 1GB
            memory_available: 512 * 1024 * 1024, // 512MB
            memory_cached: 256 * 1024 * 1024,    // 256MB
            memory_free: 256 * 1024 * 1024,      // 256MB
            timestamp: Instant::now(),
        };

        let html = StatusServer::render_html_template(&stats);

        // 检查内存值是否正确转换为 MB
        assert!(html.contains("2048")); // 总内存 2GB = 2048MB
        assert!(html.contains("1024")); // 已用内存 1GB = 1024MB
        assert!(html.contains("512")); // 可用内存 512MB
        assert!(html.contains("256")); // 缓存内存 256MB
        assert!(html.contains("256")); // 空闲内存 256MB
    }

    #[tokio::test]
    async fn test_handle_request_root() {
        let cache = create_cache(10);
        let request = Request::builder()
            .method("GET")
            .uri("/")
            .body(Body::empty())
            .unwrap();

        let response = StatusServer::handle_request(request, cache).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_handle_request_health() {
        let cache = create_cache(10);
        let request = Request::builder()
            .method("GET")
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let response = StatusServer::handle_request(request, cache).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        assert_eq!(std::str::from_utf8(&body).unwrap(), "OK");
    }

    #[tokio::test]
    async fn test_handle_request_404() {
        let cache = create_cache(10);
        let request = Request::builder()
            .method("GET")
            .uri("/notfound")
            .body(Body::empty())
            .unwrap();

        let response = StatusServer::handle_request(request, cache).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_handle_request_post_method() {
        let cache = create_cache(10);
        let request = Request::builder()
            .method("POST")
            .uri("/")
            .body(Body::empty())
            .unwrap();

        let response = StatusServer::handle_request(request, cache).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_serve_html_with_cache() {
        let cache = create_cache(10);

        // 更新缓存数据
        let stats = create_test_stats("缓存测试", 0.8);
        cache.update(stats);

        // 模拟请求处理
        let request = Request::builder()
            .method("GET")
            .uri("/")
            .body(Body::empty())
            .unwrap();

        let response = StatusServer::handle_request(request, cache.clone())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let html = std::str::from_utf8(&body).unwrap();

        // 检查响应是否包含缓存的数据
        assert!(html.contains("缓存测试"));
        assert!(html.contains("80"));
    }
}

/// 配置结构
#[derive(Debug, Clone)]
pub struct Config {
    /// 服务端绑定地址
    pub bind_address: String,
    /// 服务端端口
    pub port: u16,
    /// 缓存 TTL（秒）
    pub cache_ttl_seconds: u64,
}

impl Default for Config {
    #[inline]
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0".to_string(),
            port: 8080,
            cache_ttl_seconds: 10, // 严格 10 秒过期
        }
    }
}

impl Config {
    /// 构建服务器地址
    #[inline]
    pub fn address(&self) -> SocketAddr {
        format!("{}:{}", self.bind_address, self.port)
            .parse()
            .expect("无效的地址格式")
    }
}
