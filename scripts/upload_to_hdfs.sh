#!/bin/bash
# scripts/upload_to_hdfs.sh

DATE=${1:-$(date -d "yesterday" +%Y%m%d)}
HDFS_BASE="/bilibili/raw"
LOCAL_BASE="$HOME/output"

# 找到最新的快照目录
SNAPSHOT_DIR=$(ls -d ${LOCAL_BASE}/${DATE}_* | tail -1)

if [ -z "$SNAPSHOT_DIR" ]; then
    echo "No snapshot found for $DATE"
    exit 1
fi

echo "Uploading snapshot: $SNAPSHOT_DIR"

# 上传视频详情
hdfs dfs -mkdir -p ${HDFS_BASE}/video/date=${DATE}
hdfs dfs -put ${SNAPSHOT_DIR}/video_detail.ndjson ${HDFS_BASE}/video/date=${DATE}/

# 上传评论
hdfs dfs -mkdir -p ${HDFS_BASE}/comment/date=${DATE}
hdfs dfs -put ${SNAPSHOT_DIR}/comments/*.ndjson ${HDFS_BASE}/comment/date=${DATE}/

# 上传弹幕
hdfs dfs -mkdir -p ${HDFS_BASE}/danmaku/date=${DATE}
hdfs dfs -put ${SNAPSHOT_DIR}/danmaku/*.ndjson ${HDFS_BASE}/danmaku/date=${DATE}/

echo "Upload completed!"
