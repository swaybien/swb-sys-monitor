# 资源占用显示系统

> [English](README.en.md) | 简体中文

一个极简的资源占用显示系统，专为嵌入式设备监控场景设计。支持 200+ 高并发客户端访问。

## 特性

- **最小最简代码实现**：追求代码简洁性和可维护性；
- **高性能，内存占用低**：针对嵌入式设备优化；
- **高并发访问，无锁算法**：支持 200+ 并发连接；
- **少依赖，尽量使用系统命令**：减少外部依赖；
- **数据更新后 10 秒算过期**：数据未过期时无须再次获取；
- **无用户访问时无须获取**：按需更新策略；
- **无 CSS，无 JS**：纯 HTML 实现。

## 快速开始

### 安装

#### 下载安装预编译二进制包

```bash
cargo install swb-sys-monitor
```

#### 从源码编译

```bash
# 克隆仓库
git clone https://github.com/swaybien/swb-sys-monitor.git
cd swb-sys-monitor

# 构建发布版本
cargo build --release

# 或者直接运行
cargo run -- --help
```

### 使用

```bash
# 使用默认配置启动（监听 0.0.0.0:8080，缓存 TTL 10 秒）
./target/release/swb-sys-monitor

# 自定义配置
./target/release/swb-sys-monitor --address 127.0.0.1 --port 3000 --ttl 5

# 设置日志级别
./target/release/swb-sys-monitor --log-level debug
```

### 访问

启动后，在浏览器中访问 `http://localhost:8080` 即可查看系统资源占用情况。

页面会每 10 秒自动刷新，显示以下信息：

- 处理器使用率
- 内存使用情况（已用、可用、缓存、空闲）
- 数据获取时间戳

#### 健康检查端点

系统提供健康检查端点 `http://localhost:8080/health`，用于监控服务状态：

```bash
curl http://localhost:8080/health
# 返回: OK
```

## 命令行参数

| 参数          | 短参数 | 默认值    | 描述                                       |
| ------------- | ------ | --------- | ------------------------------------------ |
| `--address`   | `-a`   | `0.0.0.0` | 服务器绑定地址                             |
| `--port`      | `-p`   | `8080`    | 服务器端口                                 |
| `--ttl`       | `-t`   | `10`      | 缓存 TTL 秒数                              |
| `--log-level` | `-l`   | `info`    | 日志级别 (trace, debug, info, warn, error) |
| `--help`      | `-h`   | -         | 显示帮助信息                               |

## 技术架构

### 核心组件

1. **系统资源获取模块**：直接读取 `/proc/stat` 和 `/proc/meminfo` 获取系统信息
2. **无锁数据缓存机制**：使用 `AtomicPtr` 和 `AtomicU64` 实现线程安全缓存
3. **高并发 Web 服务器**：基于 tokio + hyper 实现高性能 HTTP 服务器
4. **服务器端 HTML 渲染**：纯 HTML 实现，无 CSS 无 JS

### 性能优化

- **无锁算法**：缓存读写使用原子操作，支持高并发访问
- **按需更新**：只有数据过期且有请求时才更新系统信息
- **内存优化**：使用 `String::with_capacity` 预分配容量，减少重新分配
- **函数内联**：小函数使用 `#[inline]` 属性优化性能

## 开发

### 构建要求

- Rust 2024 Edition
- Linux 系统（目前仅支持 Linux）

### 开发命令

```bash
# 编译检查
cargo check --all-targets

# 代码规范检查
cargo clippy

# 运行测试
cargo test

# 运行基准测试
cargo bench

# 构建发布版本
cargo build --release
```

### 测试覆盖

项目包含全面的测试覆盖：

- 单元测试：缓存机制、系统信息获取、HTTP 服务器
- 集成测试：端到端功能测试
- 性能基准测试：关键操作的性能测试

## 部署

### 自动安装（推荐）

使用提供的安装脚本自动部署到 systemd 系统：

```bash
# 克隆仓库
git clone https://github.com/swaybien/swb-sys-monitor.git
cd swb-sys-monitor

# 运行安装脚本
sudo ./scripts/install.sh

# 如需卸载
sudo ./scripts/uninstall.sh
```

详细的部署脚本使用说明请参考 [部署指南](scripts/README.md)。

### 手动安装

#### 系统服务配置

创建 systemd 服务文件 `/etc/systemd/system/swb-sys-monitor.service`：

```ini
[Unit]
Description=Resource Status Monitor
After=network.target

[Service]
Type=simple
User=nobody
ExecStart=/usr/local/bin/swb-sys-monitor
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

启用和启动服务：

```bash
sudo systemctl enable swb-sys-monitor
sudo systemctl start swb-sys-monitor
```

### Docker 部署

#### 使用 Docker Compose（推荐）

```bash
# 克隆仓库
git clone https://github.com/swaybien/swb-sys-monitor.git
cd swb-sys-monitor

# 启动服务
docker-compose up -d

# 查看日志
docker-compose logs -f swb-sys-monitor

# 停止服务
docker-compose down
```

访问地址：http://localhost:18273

#### 使用 Docker 命令

```bash
# 构建镜像
docker build -t swb-sys-monitor .

# 运行容器
docker run -d \
  --name swb-sys-monitor \
  --restart unless-stopped \
  -p 18273:8080 \
  --read-only \
  --tmpfs /tmp \
  --cap-drop ALL \
  --cap-add DAC_OVERRIDE \
  --security-opt no-new-privileges:true \
  swb-sys-monitor

# 查看日志
docker logs -f swb-sys-monitor

# 停止容器
docker stop swb-sys-monitor
docker rm swb-sys-monitor
```

#### Docker 配置说明

Docker 部署包含以下安全特性：

- **只读文件系统**：防止运行时修改
- **最小权限**：移除所有 Linux 能力，仅添加必要的能力
- **安全选项**：启用 `no-new-privileges` 防止权限提升
- **健康检查**：自动监控服务状态
- **非 root 用户**：容器内使用非特权用户运行

#### 自定义配置

可以通过环境变量自定义配置：

```yaml
# docker-compose.yml
services:
  swb-sys-monitor:
    build: .
    environment:
      - RUST_LOG=debug
    command:
      [
        "./swb-sys-monitor",
        "--address",
        "0.0.0.0",
        "--port",
        "8080",
        "--ttl",
        "5",
      ]
```

## 许可证

本项目采用 MIT 许可证。详见 [LICENSE](LICENSE) 文件。

## 贡献

欢迎提交 Issue 和 Pull Request！
请阅读[贡献指北](CONTRIBUTING.md)。

## 相关链接

- [设计文档](docs/design.md)
- [开发状态](docs/status.md)
- [HTML 模板示例](docs/example.html)
