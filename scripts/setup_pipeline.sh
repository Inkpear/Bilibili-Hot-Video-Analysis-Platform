#!/bin/bash
# setup_pipeline.sh — 首次初始化数据管线
#
# 在 WSL 上运行，需要能 SSH 到 master (hadoop@192.168.100.10)
# 同时设置 Hive ODS 外部表 + MySQL ADS 表

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
MASTER_HOST="192.168.100.10"
HADOOP_USER="hadoop"
HADOOP_PASS="hadoop123"
SSH_CMD="sshpass -p ${HADOOP_PASS} ssh -o StrictHostKeyChecking=no ${HADOOP_USER}@${MASTER_HOST}"

echo "================================================"
echo "  数据管线首次初始化"
echo "================================================"

# 1. 创建 HDFS 目录
echo ""
echo "[1/4] 创建 HDFS 目录..."
$SSH_CMD "
    hdfs dfs -mkdir -p /data/bilibili/ods/rank_video/
    hdfs dfs -mkdir -p /data/bilibili/ods/video_comment/
    hdfs dfs -mkdir -p /data/bilibili/ods/video_danmaku/
    hdfs dfs -mkdir -p /data/bilibili/ods/up_info/
    hdfs dfs -chmod -R 755 /data/bilibili/
"
echo "  HDFS 目录已创建"

# 2. 创建 Hive ODS 外部表
echo ""
echo "[2/4] 创建 Hive ODS 外部表..."
$SSH_CMD "hive -f /home/hadoop/sql/hive_ods_ddl.sql" 2>&1 | tail -5 || {
    echo "  通过 hive CLI 执行失败，尝试复制 SQL 到 master"
    cat "$SCRIPT_DIR/../sql/hive_ods_ddl.sql" | $SSH_CMD "cat > /tmp/hive_ods_ddl.sql && hive -f /tmp/hive_ods_ddl.sql" 2>&1 | tail -5
}
echo "  Hive ODS 表已创建"

# 3. 创建 MySQL ADS 表
echo ""
echo "[3/4] 创建 MySQL ADS 表..."
cat "$SCRIPT_DIR/../sql/mysql_ads_ddl.sql" | $SSH_CMD \
    "mysql -u hive -pHive@123456" 2>&1 | tail -5
echo "  MySQL ADS 表已创建"

# 4. 复制 SQL 到 master
echo ""
echo "[4/4] 复制 ETL SQL 到 master..."
cat "$SCRIPT_DIR/../sql/spark_etl.sql" | $SSH_CMD "cat > /home/hadoop/sql/spark_etl.sql"
echo "  ETL SQL 已部署"

echo ""
echo "================================================"
echo "  初始化完成"
echo "================================================"
echo ""
echo "  后续日常流程:"
echo "    1) crawler 每天 00:00 自动运行"
echo "    2) bash scripts/run_pipeline.sh  手动触发 ETL"
echo "    3) 或添加到 crawler.service 的 ExecStartPost"
