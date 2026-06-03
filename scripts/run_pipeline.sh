#!/bin/bash
# run_pipeline.sh — 日常 ETL 管道
#
# 在爬虫采集完成后运行:
#   1. 上传最新 NDJSON 到 HDFS
#   2. 运行 Spark ETL (ODS → DWD → DWS → ADS)
#   3. 运行标题词云 MapReduce
#   4. 导出 ADS 到 MySQL
#
# 用法:
#   bash run_pipeline.sh                    # 处理最新 snapshot
#   bash run_pipeline.sh 20260601_0000      # 处理指定 snapshot

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
MASTER_HOST="192.168.100.10"
HADOOP_USER="hadoop"
HADOOP_PASS="hadoop123"
SSH_CMD="sshpass -p ${HADOOP_PASS} ssh -o StrictHostKeyChecking=no ${HADOOP_USER}@${MASTER_HOST}"

echo "================================================"
echo "  数据管线 ETL — $(date '+%Y-%m-%d %H:%M:%S')"
echo "================================================"

# Step 0: 健康检查 — 确保 HDFS、Hive Metastore、HiveServer2 可用
echo ""
echo "[0/4] 集群健康检查..."

check_port() {
    local host="$1" port="$2" name="$3"
    if ! $SSH_CMD "netstat -tlnp 2>/dev/null | grep -q ':${port} '" 2>/dev/null; then
        echo "  [WARN] ${name} (port ${port}) 不可达，尝试启动..."
        return 1
    fi
    echo "  [OK] ${name} (port ${port})"
    return 0
}

# 检查 HDFS
if ! $SSH_CMD "hdfs dfsadmin -report" >/dev/null 2>&1; then
    echo "  [WARN] HDFS 不可用，尝试重启..."
    $SSH_CMD "start-dfs.sh" 2>&1 | tail -3
    sleep 5
fi
echo "  [OK] HDFS"

# 检查 Hive Metastore (9083)
if ! check_port "$MASTER_HOST" 9083 "Hive Metastore"; then
    $SSH_CMD "echo '${HADOOP_PASS}' | sudo -S systemctl start hive-metastore.service" 2>&1
    sleep 10
    check_port "$MASTER_HOST" 9083 "Hive Metastore" || { echo "  [FAIL] Hive Metastore 启动失败"; exit 1; }
fi

# 检查 HiveServer2 (10000)
if ! check_port "$MASTER_HOST" 10000 "HiveServer2"; then
    $SSH_CMD "echo '${HADOOP_PASS}' | sudo -S systemctl start hive-server2.service" 2>&1
    sleep 30
    check_port "$MASTER_HOST" 10000 "HiveServer2" || { echo "  [FAIL] HiveServer2 启动失败"; exit 1; }
fi

echo "  集群健康检查通过"

# Step 1: 上传 NDJSON 到 HDFS
echo ""
echo "[1/4] 上传 NDJSON 到 HDFS..."
bash "$SCRIPT_DIR/upload_hdfs.sh" "$@"

# Step 2: 运行 Spark ETL
echo ""
echo "[2/4] 运行 Spark ETL..."
$SSH_CMD "spark-sql --master spark://master:7077 -f /home/hadoop/sql/spark_etl.sql" 2>&1 | tail -20

# Step 3: 词云 MapReduce（标题/评论/弹幕）
echo ""
echo "[3/4] 运行词云 MapReduce..."
$SSH_CMD "SOURCE_TYPE=title bash /home/hadoop/scripts/run_tag_cloud_mr.sh" 2>&1 | tail -5
$SSH_CMD "SOURCE_TYPE=comment bash /home/hadoop/scripts/run_tag_cloud_mr.sh" 2>&1 | tail -5
$SSH_CMD "SOURCE_TYPE=danmaku bash /home/hadoop/scripts/run_tag_cloud_mr.sh" 2>&1 | tail -5

echo ""
echo "[3.5/4] 合并词云到 ads_word_cloud..."
$SSH_CMD "spark-sql --master spark://master:7077 -e \"
USE bilibili;
INSERT OVERWRITE TABLE ads_word_cloud
SELECT source_type, category, word, word_count
FROM (
  SELECT source_type, category, word, word_count,
         ROW_NUMBER() OVER (
           PARTITION BY source_type, category ORDER BY word_count DESC, word ASC
         ) AS rn
  FROM (
    SELECT 'title' AS source_type,
           CASE WHEN category = '__ALL__' THEN '全站' ELSE category END AS category,
           word, word_count
    FROM mr_title_raw
    WHERE word IS NOT NULL AND word != '' AND word_count >= 2
    UNION ALL
    SELECT 'comment' AS source_type,
           CASE WHEN category = '__ALL__' THEN '全站' ELSE category END AS category,
           word, word_count
    FROM mr_comment_raw
    WHERE word IS NOT NULL AND word != '' AND word_count >= 2
    UNION ALL
    SELECT 'danmaku' AS source_type,
           CASE WHEN category = '__ALL__' THEN '全站' ELSE category END AS category,
           word, word_count
    FROM mr_danmaku_raw
    WHERE word IS NOT NULL AND word != '' AND word_count >= 2
  ) all_words
) ranked
WHERE (category = '全站' AND rn <= 150)
   OR (category != '全站' AND rn <= 100);
\"" 2>&1 | tail -5

# Step 4: 导出到 MySQL
echo ""
echo "[4/4] 导出 ADS 到 MySQL..."
$SSH_CMD "spark-submit --jars /opt/spark/jars/mysql-connector-j-8.0.33.jar \
    --master spark://master:7077 \
    /home/hadoop/scripts/export_to_mysql.py" 2>&1 | tail -15

echo ""
echo "================================================"
echo "  管线完成 — $(date '+%Y-%m-%d %H:%M:%S')"
echo "================================================"
