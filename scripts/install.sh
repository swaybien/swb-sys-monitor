#!/bin/bash

# 资源占用显示系统安装脚本
# 支持 systemd 系统的自动安装和配置

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 打印函数
print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 检查是否为 root 用户
check_root() {
    if [[ $EUID -ne 0 ]]; then
        print_error "此脚本需要 root 权限运行"
        exit 1
    fi
}

# 检查系统类型
check_system() {
    if ! command -v systemctl &> /dev/null; then
        print_error "此脚本仅支持 systemd 系统"
        exit 1
    fi
    
    if [[ ! -f /etc/os-release ]]; then
        print_error "无法检测操作系统类型"
        exit 1
    fi
    
    source /etc/os-release
    print_info "检测到操作系统: $NAME $VERSION"
}

# 检查 Rust 环境
check_rust() {
    if ! command -v cargo &> /dev/null; then
        print_warn "未检测到 Rust 环境，正在安装..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source $HOME/.cargo/env
    else
        print_info "检测到 Rust 环境: $(rustc --version)"
    fi
}

# 创建系统用户
create_user() {
    if ! id "nobody" &>/dev/null; then
        print_info "创建系统用户 nobody"
        useradd --system --no-create-home --shell /usr/sbin/nologin nobody || true
    else
        print_info "系统用户 nobody 已存在"
    fi
}

# 编译项目
build_project() {
    print_info "开始编译项目..."
    
    if [[ -d "swb-sys-monitor" ]]; then
        cd swb-sys-monitor
    else
        print_error "项目目录不存在"
        exit 1
    fi
    
    print_info "运行 cargo build --release..."
    cargo build --release
    
    if [[ ! -f "target/release/swb-sys-monitor" ]]; then
        print_error "编译失败"
        exit 1
    fi
    
    print_info "编译成功"
}

# 安装二进制文件
install_binary() {
    print_info "安装二进制文件到 /usr/local/bin/"
    
    # 停止可能正在运行的服务
    systemctl stop swb-sys-monitor 2>/dev/null || true
    systemctl disable swb-sys-monitor 2>/dev/null || true
    
    # 复制二进制文件
    cp target/release/swb-sys-monitor /usr/local/bin/
    chmod +x /usr/local/bin/swb-sys-monitor
    
    print_info "二进制文件安装完成"
}

# 安装系统服务
install_service() {
    print_info "安装 systemd 服务..."
    
    # 复制服务文件
    cp scripts/swb-sys-monitor.service /etc/systemd/system/
    
    # 重新加载 systemd
    systemctl daemon-reload
    
    # 启用服务
    systemctl enable swb-sys-monitor
    
    print_info "系统服务安装完成"
}

# 启动服务
start_service() {
    print_info "启动 swb-sys-monitor 服务..."
    
    systemctl start swb-sys-monitor
    
    # 等待服务启动
    sleep 2
    
    # 检查服务状态
    if systemctl is-active --quiet swb-sys-monitor; then
        print_info "服务启动成功"
        print_info "服务状态: $(systemctl is-active swb-sys-monitor)"
    else
        print_error "服务启动失败"
        print_error "服务日志:"
        journalctl -u swb-sys-monitor --no-pager -l
        exit 1
    fi
}

# 显示使用信息
show_usage() {
    print_info "安装完成！"
    echo
    print_info "服务管理命令:"
    echo "  启动服务: systemctl start swb-sys-monitor"
    echo "  停止服务: systemctl stop swb-sys-monitor"
    echo "  重启服务: systemctl restart swb-sys-monitor"
    echo "  查看状态: systemctl status swb-sys-monitor"
    echo "  查看日志: journalctl -u swb-sys-monitor -f"
    echo
    print_info "访问地址: http://localhost:8080"
    print_info "健康检查: http://localhost:8080/health"
    echo
    print_info "配置文件位置: /etc/systemd/system/swb-sys-monitor.service"
    print_info "如需修改启动参数，请编辑服务文件后运行:"
    echo "  systemctl daemon-reload"
    echo "  systemctl restart swb-sys-monitor"
}

# 主函数
main() {
    print_info "开始安装资源占用显示系统..."
    
    check_root
    check_system
    check_rust
    create_user
    
    # 获取项目目录
    SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
    PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
    cd "$PROJECT_DIR"
    
    build_project
    install_binary
    install_service
    start_service
    show_usage
    
    print_info "安装完成！"
}

# 运行主函数
main "$@"