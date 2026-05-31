#!/bin/bash
# deploy.sh — 一键部署 crawler 到 WSL
#
# 用法 (在 Mac 上执行):
#   ./deploy.sh                                    # 部署到 WSL
#   ./deploy.sh --cookie "SESSDATA=..."            # 直接指定 Cookie
#   ./deploy.sh --env-file ../scripts/.env         # 从 .env 文件读取 Cookie
#
# 前提:
#   - Mac 上已配置 ssh UbuntuDesktop 可连通 WSL
#   - WSL 已安装 python3, pip3, systemd
#   - WSL 上 /home/inkpear 目录存在

set -euo pipefail

WSL_HOST="${WSL_HOST:-UbuntuDesktop}"
WSL_USER="${WSL_USER:-inkpear}"
WSL_HOME="/home/${WSL_USER}"
WSL_CRAWLER_DIR="${WSL_HOME}/crawler"
WSL_SCRIPTS_DIR="${WSL_HOME}/scripts"
WSL_SQL_DIR="${WSL_HOME}/sql"
WSL_MAPREDUCE_DIR="${WSL_HOME}/mapreduce"
WSL_LOGS_DIR="${WSL_CRAWLER_DIR}/logs"
WSL_OUTPUT_DIR="${WSL_CRAWLER_DIR}/../output"

COOKIE=""
ENV_FILE=""

usage() {
    echo "用法: $0 [--cookie COOKIE_STRING] [--env-file .env路径]"
    exit 1
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --cookie)
            COOKIE="$2"; shift 2 ;;
        --env-file)
            ENV_FILE="$2"; shift 2 ;;
        --help|-h)
            usage ;;
        *)
            echo "未知参数: $1"; usage ;;
    esac
done

# 获取 Cookie
if [ -z "$COOKIE" ] && [ -n "$ENV_FILE" ]; then
    if [ -f "$ENV_FILE" ]; then
        COOKIE=$(grep -o 'BILI_COOKIE=".*"' "$ENV_FILE" | sed 's/BILI_COOKIE="//;s/"$//')
        echo "从 $ENV_FILE 读取 Cookie (${#COOKIE} 字符)"
    else
        echo "错误: env 文件不存在: $ENV_FILE"
        exit 1
    fi
fi

if [ -z "$COOKIE" ]; then
    echo "错误: 请指定 --cookie 或 --env-file"
    echo "  例: $0 --cookie \"SESSDATA=...; bili_jct=...; DedeUserID=...\""
    echo "  例: $0 --env-file ../scripts/.env"
    exit 1
fi

echo "================================================"
echo "  部署 B站 crawler 到 WSL"
echo "================================================"
echo "  目标: ${WSL_USER}@${WSL_HOST}:${WSL_CRAWLER_DIR}"
echo ""

# ------------------------------------------------------------------
# Step 1: 创建目录结构
# ------------------------------------------------------------------
echo "[1/5] 创建目录..."
ssh "$WSL_HOST" "mkdir -p ${WSL_CRAWLER_DIR} ${WSL_SCRIPTS_DIR} ${WSL_SQL_DIR} ${WSL_MAPREDUCE_DIR} ${WSL_LOGS_DIR} ${WSL_OUTPUT_DIR}"

# ------------------------------------------------------------------
# Step 2: 同步 crawler 源码和管线脚本
# ------------------------------------------------------------------
echo "[2/5] 同步源码..."
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CRAWLER_DIR="$(cd "$SCRIPT_DIR/../crawler" && pwd)"
SCRIPTS_DIR="$(cd "$SCRIPT_DIR" && pwd)"
SQL_DIR="$(cd "$SCRIPT_DIR/../sql" && pwd)"
MAPREDUCE_DIR="$(cd "$SCRIPT_DIR/../mapreduce" && pwd)"
rsync -avz --delete \
    --exclude='__pycache__' \
    --exclude='*.pyc' \
    --exclude='.DS_Store' \
    --exclude='crawler_state.db*' \
    --exclude='logs/' \
    --exclude='output/' \
    "$CRAWLER_DIR"/ \
    "$WSL_HOST:${WSL_CRAWLER_DIR}/"
rsync -avz --delete \
    --exclude='__pycache__' \
    --exclude='*.pyc' \
    --exclude='.DS_Store' \
    --exclude='.env' \
    "$SCRIPTS_DIR"/ \
    "$WSL_HOST:${WSL_SCRIPTS_DIR}/"
rsync -avz --delete \
    --exclude='.DS_Store' \
    "$SQL_DIR"/ \
    "$WSL_HOST:${WSL_SQL_DIR}/"
rsync -avz --delete \
    --exclude='__pycache__' \
    --exclude='*.pyc' \
    --exclude='.DS_Store' \
    "$MAPREDUCE_DIR"/ \
    "$WSL_HOST:${WSL_MAPREDUCE_DIR}/"

# ------------------------------------------------------------------
# Step 3: 写入 .env
# ------------------------------------------------------------------
echo "[3/5] 配置环境变量..."
ssh "$WSL_HOST" "echo 'BILI_COOKIE=\"${COOKIE}\"' > ${WSL_CRAWLER_DIR}/.env"
ssh "$WSL_HOST" "chmod 600 ${WSL_CRAWLER_DIR}/.env"
echo "  .env 已写入 (${#COOKIE} 字符)"

# ------------------------------------------------------------------
# Step 4: 安装 Python 依赖
# ------------------------------------------------------------------
echo "[4/5] 安装依赖..."
ssh "$WSL_HOST" "cd ${WSL_CRAWLER_DIR} && python3 -c 'import requests, tqdm' || python3 -m pip install --break-system-packages -q -r requirements.txt"
echo "  依赖安装完成"

# ------------------------------------------------------------------
# Step 5: 安装 user-level systemd 服务
# ------------------------------------------------------------------
echo "[5/5] 安装 systemd 定时任务..."
ssh "$WSL_HOST" << 'ENDSSH'
    mkdir -p "$HOME/.config/systemd/user"
    cp "$HOME/crawler/crawler.service" "$HOME/.config/systemd/user/"
    cp "$HOME/crawler/crawler.timer" "$HOME/.config/systemd/user/"
    systemctl --user daemon-reload
    systemctl --user enable crawler.timer
    systemctl --user restart crawler.timer
ENDSSH

echo ""
echo "================================================"
echo "  部署完成!"
echo "================================================"
echo ""
echo "  管理命令:"
echo "    systemctl --user status crawler.timer    # 查看定时器状态"
echo "    systemctl --user start crawler.service   # 手动触发一次采集"
echo "    journalctl --user -u crawler.service -f  # 查看实时日志"
echo "    journalctl --user -u crawler.service --since today --no-pager"
echo ""
echo "  下次触发时间:"
ssh "$WSL_HOST" "systemctl --user list-timers crawler.timer --no-pager 2>/dev/null || echo '  (查看失败)'"
