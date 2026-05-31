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

# Step 1: 上传 NDJSON 到 HDFS
echo ""
echo "[1/4] 上传 NDJSON 到 HDFS..."
bash "$SCRIPT_DIR/upload_hdfs.sh" "$@"

# Step 2: 运行 Spark ETL
echo ""
echo "[2/4] 运行 Spark ETL..."
$SSH_CMD "spark-sql --master spark://master:7077 -f /home/hadoop/sql/spark_etl.sql" 2>&1 | tail -20

# Step 3: 标题词云 MapReduce
echo ""
echo "[3/4] 运行标题词云 MapReduce..."
bash "$SCRIPT_DIR/run_tag_cloud_mr.sh"

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
