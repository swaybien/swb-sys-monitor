# 部署脚本使用指南

本目录包含了资源占用显示系统的部署脚本，用于简化安装和卸载过程。

## 文件说明

- `install.sh` - 自动安装脚本，适用于 systemd 系统
- `uninstall.sh` - 卸载脚本，完全移除系统中的服务
- `swb-sys-monitor.service` - systemd 服务配置文件

## 安装脚本使用方法

### 前置要求

- Linux 系统（支持 systemd）
- Root 权限
- 网络连接（用于安装 Rust 依赖）

### 安装步骤

1. 确保脚本有执行权限：
   ```bash
   chmod +x scripts/install.sh
   ```

2. 运行安装脚本：
   ```bash
   sudo ./scripts/install.sh
   ```

3. 安装脚本会自动完成以下操作：
   - 检查系统环境
   - 安装 Rust 工具链（如果未安装）
   - 编译项目
   - 安装二进制文件到 `/usr/local/bin/`
   - 配置 systemd 服务
   - 启动服务

### 验证安装

安装完成后，可以通过以下方式验证：

```bash
# 检查服务状态
systemctl status swb-sys-monitor

# 查看服务日志
journalctl -u swb-sys-monitor -f

# 访问健康检查端点
curl http://localhost:8080/health

# 访问主页面
curl http://localhost:8080
```

## 卸载脚本使用方法

### 卸载步骤

1. 确保脚本有执行权限：
   ```bash
   chmod +x scripts/uninstall.sh
   ```

2. 运行卸载脚本：
   ```bash
   sudo ./scripts/uninstall.sh
   ```

3. 卸载脚本会：
   - 停止并禁用服务
   - 删除 systemd 服务文件
   - 删除二进制文件
   - 可选：清理服务日志

## 服务管理

### 常用命令

```bash
# 启动服务
sudo systemctl start swb-sys-monitor

# 停止服务
sudo systemctl stop swb-sys-monitor

# 重启服务
sudo systemctl restart swb-sys-monitor

# 查看服务状态
sudo systemctl status swb-sys-monitor

# 启用开机自启
sudo systemctl enable swb-sys-monitor

# 禁用开机自启
sudo systemctl disable swb-sys-monitor
```

### 查看日志

```bash
# 查看实时日志
sudo journalctl -u swb-sys-monitor -f

# 查看最近的日志
sudo journalctl -u swb-sys-monitor --since "1 hour ago"

# 查看错误日志
sudo journalctl -u swb-sys-monitor -p err
```

## 自定义配置

### 修改启动参数

编辑服务配置文件：
```bash
sudo nano /etc/systemd/system/swb-sys-monitor.service
```

修改 `ExecStart` 行，添加所需参数：
```ini
ExecStart=/usr/local/bin/swb-sys-monitor --address 127.0.0.1 --port 3000 --ttl 5 --log-level debug
```

重新加载并重启服务：
```bash
sudo systemctl daemon-reload
sudo systemctl restart swb-sys-monitor
```

### 修改端口

如果需要修改监听端口，请同时修改：

1. 服务配置文件中的 `ExecStart` 参数
2. 防火墙规则（如果启用）

```bash
# 例如：开放 3000 端口
sudo ufw allow 3000
```

## 故障排除

### 常见问题

1. **服务启动失败**
   ```bash
   # 查看详细错误信息
   sudo journalctl -u swb-sys-monitor -n 50
   ```

2. **端口被占用**
   ```bash
   # 查看端口占用情况
   sudo netstat -tlnp | grep :8080
   # 或使用 ss 命令
   sudo ss -tlnp | grep :8080
   ```

3. **权限问题**
   ```bash
   # 检查二进制文件权限
   ls -la /usr/local/bin/swb-sys-monitor
   
   # 修复权限
   sudo chmod +x /usr/local/bin/swb-sys-monitor
   ```

### 手动测试

如果服务无法正常工作，可以手动运行测试：

```bash
# 切换到 nobody 用户（服务运行用户）
sudo -u nobody /usr/local/bin/swb-sys-monitor --log-level debug
```

## 安全注意事项

- 服务以 `nobody` 用户运行，具有最小权限
- 只允许访问必要的系统文件（`/proc/stat`、`/proc/meminfo`、`/proc/sys/kernel/hostname`）
- 使用 systemd 的安全限制（`ReadOnlyPaths`、`ProtectSystem` 等）
- 建议在生产环境中配置防火墙规则
