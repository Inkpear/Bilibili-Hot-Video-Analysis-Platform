#!/bin/bash
# crawler_loop.sh — 常驻采集循环：立即采集，完成后休眠 6 小时再采。

set -uo pipefail

SLEEP_SECONDS="${CRAWLER_SLEEP_SECONDS:-21600}"
CRAWLER_DIR="${CRAWLER_DIR:-$HOME/crawler}"
SCRIPTS_DIR="${SCRIPTS_DIR:-$HOME/scripts}"

stop_requested=0

handle_stop() {
    stop_requested=1
    echo "收到停止信号，当前轮结束后退出 — $(date '+%Y-%m-%d %H:%M:%S')"
}

trap handle_stop TERM INT

echo "================================================"
echo "  B站 crawler 常驻循环启动 — $(date '+%Y-%m-%d %H:%M:%S')"
echo "  休眠间隔: ${SLEEP_SECONDS}s"
echo "================================================"

while [ "$stop_requested" -eq 0 ]; do
    echo ""
    echo "[crawler-loop] 开始采集 — $(date '+%Y-%m-%d %H:%M:%S')"
    cd "$CRAWLER_DIR" || exit 1

    /usr/bin/python3 main.py --once
    crawler_status=$?

    if [ "$crawler_status" -eq 0 ]; then
        echo "[crawler-loop] 采集完成，开始 ETL — $(date '+%Y-%m-%d %H:%M:%S')"
        /bin/bash "$SCRIPTS_DIR/run_pipeline.sh"
        pipeline_status=$?
        if [ "$pipeline_status" -eq 0 ]; then
            echo "[crawler-loop] ETL 完成 — $(date '+%Y-%m-%d %H:%M:%S')"
        else
            echo "[crawler-loop] ETL 失败 status=${pipeline_status} — $(date '+%Y-%m-%d %H:%M:%S')"
        fi
    else
        echo "[crawler-loop] 采集失败 status=${crawler_status}，跳过本轮 ETL — $(date '+%Y-%m-%d %H:%M:%S')"
    fi

    if [ "$stop_requested" -ne 0 ]; then
        break
    fi

    echo "[crawler-loop] 休眠 ${SLEEP_SECONDS}s，等待下一轮 — $(date '+%Y-%m-%d %H:%M:%S')"
    sleep "$SLEEP_SECONDS" &
    wait $!
done

echo "[crawler-loop] 已退出 — $(date '+%Y-%m-%d %H:%M:%S')"
