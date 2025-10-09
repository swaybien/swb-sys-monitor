#!/bin/bash

# 资源占用显示系统卸载脚本

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

# 停止并禁用服务
stop_service() {
    print_info "停止并禁用 swb-sys-monitor 服务..."
    
    if systemctl is-active --quiet swb-sys-monitor 2>/dev/null; then
        systemctl stop swb-sys-monitor
        print_info "服务已停止"
    else
        print_info "服务未运行"
    fi
    
    if systemctl is-enabled --quiet swb-sys-monitor 2>/dev/null; then
        systemctl disable swb-sys-monitor
        print_info "服务已禁用"
    else
        print_info "服务未启用"
    fi
}

# 删除系统服务文件
remove_service() {
    print_info "删除系统服务文件..."
    
    if [[ -f /etc/systemd/system/swb-sys-monitor.service ]]; then
        rm -f /etc/systemd/system/swb-sys-monitor.service
        systemctl daemon-reload
        print_info "系统服务文件已删除"
    else
        print_info "系统服务文件不存在"
    fi
}

# 删除二进制文件
remove_binary() {
    print_info "删除二进制文件..."
    
    if [[ -f /usr/local/bin/swb-sys-monitor ]]; then
        rm -f /usr/local/bin/swb-sys-monitor
        print_info "二进制文件已删除"
    else
        print_info "二进制文件不存在"
    fi
}

# 清理日志文件（可选）
clean_logs() {
    read -p "是否要清理服务日志？[y/N]: " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        print_info "清理服务日志..."
        journalctl --vacuum-time=1d -u swb-sys-monitor 2>/dev/null || true
        print_info "日志已清理"
    fi
}

# 确认卸载
confirm_uninstall() {
    echo
    print_warn "即将卸载资源占用显示系统"
    print_warn "这将删除："
    echo "  - 系统服务文件"
    echo "  - 二进制文件"
    echo "  - 可选：服务日志"
    echo
    read -p "确认卸载？[y/N]: " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        print_info "卸载已取消"
        exit 0
    fi
}

# 主函数
main() {
    print_info "开始卸载资源占用显示系统..."
    
    check_root
    confirm_uninstall
    
    stop_service
    remove_service
    remove_binary
    clean_logs
    
    print_info "卸载完成！"
    echo
    print_info "资源占用显示系统已从您的系统中移除"
    print_info "如需重新安装，请运行: sudo ./scripts/install.sh"
}

# 运行主函数
main "$@"