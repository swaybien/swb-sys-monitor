# 多阶段构建 Dockerfile for 资源占用显示系统
# 使用 Rust 官方镜像作为构建环境
FROM rust:1.75-alpine as builder

# 设置工作目录
WORKDIR /app

# 安装构建依赖
RUN apk add --no-cache musl-dev

# 复制 Cargo 文件
COPY Cargo.toml Cargo.lock ./

# 创建虚拟 main.rs 以预构建依赖
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release && rm -rf src

# 复制源代码
COPY . .

# 构建应用
RUN touch src/main.rs && cargo build --release

# 运行时镜像 - 使用轻量级 Alpine
FROM alpine:latest

# 安装运行时依赖
RUN apk add --no-cache ca-certificates tzdata

# 创建非 root 用户
RUN addgroup -g 1000 -S swb && \
    adduser -D -s /bin/sh -u 1000 -S swb -G swb

# 设置工作目录
WORKDIR /app

# 从构建阶段复制二进制文件
COPY --from=builder /app/target/release/swb-sys-monitor ./

# 设置权限
RUN chown -R swb:swb /app

# 切换到非 root 用户
USER swb

# 暴露端口
EXPOSE 8080

# 设置环境变量
ENV RUST_LOG=info

# 健康检查
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8080/health || exit 1

# 启动命令
CMD ["./swb-sys-monitor", "--address", "0.0.0.0", "--port", "8080", "--log-level", "info"]