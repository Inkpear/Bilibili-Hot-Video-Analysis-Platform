#!/bin/bash
# upload_hdfs.sh — 上传 WSL 本地 NDJSON 到 HDFS
#
# 用法:
#   bash upload_hdfs.sh                           # 上传最新 snapshot
#   bash upload_hdfs.sh 20260601_0000             # 上传指定 snapshot
#
# 前提:
#   - WSL 已安装 sshpass（密码认证）
#   - HDFS 在 master:192.168.100.10，hadoop 用户密码 hadoop123
#   - 本地 output/ 目录有采集数据

set -euo pipefail

MASTER_HOST="${MASTER_HOST:-192.168.100.10}"
MASTER_USER="${MASTER_USER:-hadoop}"
MASTER_PASS="${MASTER_PASS:-hadoop123}"
OUTPUT_DIR="${OUTPUT_DIR:-$HOME/output}"
HDFS_BASE="/data/bilibili/ods"

# 找到要上传的 snapshot
if [ $# -ge 1 ]; then
    SNAPSHOT_ID="$1"
else
    SNAPSHOT_ID=$(ls -1 "$OUTPUT_DIR" 2>/dev/null | sort | tail -1)
fi

if [ -z "$SNAPSHOT_ID" ]; then
    echo "错误: 没有找到 snapshot 数据"
    exit 1
fi

SNAPSHOT_DIR="$OUTPUT_DIR/$SNAPSHOT_ID"
if [ ! -d "$SNAPSHOT_DIR" ]; then
    echo "错误: 目录不存在: $SNAPSHOT_DIR"
    exit 1
fi

echo "================================================"
echo "  上传 snapshot: $SNAPSHOT_ID"
echo "  目标: ${MASTER_USER}@${MASTER_HOST}:${HDFS_BASE}"
echo "================================================"

sshpass -p "$MASTER_PASS" ssh -o StrictHostKeyChecking=no ${MASTER_USER}@${MASTER_HOST} "
    hdfs dfs -mkdir -p ${HDFS_BASE}/rank_video/
    hdfs dfs -mkdir -p ${HDFS_BASE}/video_comment/
    hdfs dfs -mkdir -p ${HDFS_BASE}/video_danmaku/
    hdfs dfs -mkdir -p ${HDFS_BASE}/up_info/
"

# 上传 rank_video.ndjson
if [ -f "$SNAPSHOT_DIR/rank_video.ndjson" ]; then
    echo "[1/4] 上传 rank_video.ndjson ..."
    cat "$SNAPSHOT_DIR/rank_video.ndjson" | \
        sshpass -p "$MASTER_PASS" ssh -o StrictHostKeyChecking=no ${MASTER_USER}@${MASTER_HOST} \
        "hdfs dfs -put - ${HDFS_BASE}/rank_video/${SNAPSHOT_ID}_rank_video.ndjson"
fi

# 上传评论
if [ -d "$SNAPSHOT_DIR/comments" ] && [ "$(ls -A $SNAPSHOT_DIR/comments 2>/dev/null)" ]; then
    echo "[2/4] 上传评论 ($(ls $SNAPSHOT_DIR/comments | wc -l) 个文件) ..."
    for f in "$SNAPSHOT_DIR"/comments/*.ndjson; do
        bvid=$(basename "$f" .ndjson)
        cat "$f" | \
            sshpass -p "$MASTER_PASS" ssh -o StrictHostKeyChecking=no ${MASTER_USER}@${MASTER_HOST} \
            "hdfs dfs -put - ${HDFS_BASE}/video_comment/${SNAPSHOT_ID}_${bvid}.ndjson"
    done
fi

# 上传弹幕
if [ -d "$SNAPSHOT_DIR/danmaku" ] && [ "$(ls -A $SNAPSHOT_DIR/danmaku 2>/dev/null)" ]; then
    echo "[3/4] 上传弹幕 ($(ls $SNAPSHOT_DIR/danmaku | wc -l) 个文件) ..."
    for f in "$SNAPSHOT_DIR"/danmaku/*.ndjson; do
        bvid=$(basename "$f" .ndjson)
        cat "$f" | \
            sshpass -p "$MASTER_PASS" ssh -o StrictHostKeyChecking=no ${MASTER_USER}@${MASTER_HOST} \
            "hdfs dfs -put - ${HDFS_BASE}/video_danmaku/${SNAPSHOT_ID}_${bvid}.ndjson"
    done
fi

# 上传 UP 主
if [ -f "$SNAPSHOT_DIR/up_info.ndjson" ]; then
    echo "[4/4] 上传 up_info.ndjson ..."
    cat "$SNAPSHOT_DIR/up_info.ndjson" | \
        sshpass -p "$MASTER_PASS" ssh -o StrictHostKeyChecking=no ${MASTER_USER}@${MASTER_HOST} \
        "hdfs dfs -put - ${HDFS_BASE}/up_info/${SNAPSHOT_ID}_up_info.ndjson"
fi

echo ""
echo "上传完成: $SNAPSHOT_ID"
hdfs dfs -ls -R ${HDFS_BASE}/ 2>/dev/null | tail -10
