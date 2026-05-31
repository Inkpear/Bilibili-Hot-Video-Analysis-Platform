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

# 0. 部署 SQL 和导出脚本
echo ""
echo "[0/4] 部署 SQL、MapReduce 和导出脚本..."
$SSH_CMD "mkdir -p /home/hadoop/sql /home/hadoop/scripts /home/hadoop/mapreduce"
cat "$SCRIPT_DIR/../sql/hive_ods_ddl.sql" | $SSH_CMD "cat > /home/hadoop/sql/hive_ods_ddl.sql"
cat "$SCRIPT_DIR/../sql/mysql_ads_ddl.sql" | $SSH_CMD "cat > /home/hadoop/sql/mysql_ads_ddl.sql"
cat "$SCRIPT_DIR/../sql/spark_etl.sql" | $SSH_CMD "cat > /home/hadoop/sql/spark_etl.sql"
cat "$SCRIPT_DIR/export_to_mysql.py" | $SSH_CMD "cat > /home/hadoop/scripts/export_to_mysql.py && chmod +x /home/hadoop/scripts/export_to_mysql.py"
cat "$SCRIPT_DIR/run_tag_cloud_mr.sh" | $SSH_CMD "cat > /home/hadoop/scripts/run_tag_cloud_mr.sh && chmod +x /home/hadoop/scripts/run_tag_cloud_mr.sh"
cat "$SCRIPT_DIR/../mapreduce/mapper.py" | $SSH_CMD "cat > /home/hadoop/mapreduce/mapper.py && chmod +x /home/hadoop/mapreduce/mapper.py"
cat "$SCRIPT_DIR/../mapreduce/reducer.py" | $SSH_CMD "cat > /home/hadoop/mapreduce/reducer.py && chmod +x /home/hadoop/mapreduce/reducer.py"
cat "$SCRIPT_DIR/../mapreduce/stopwords.txt" | $SSH_CMD "cat > /home/hadoop/mapreduce/stopwords.txt"
echo "  SQL、MapReduce 和导出脚本已部署"

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
$SSH_CMD "hive -f /home/hadoop/sql/hive_ods_ddl.sql" 2>&1 | tail -5
echo "  Hive ODS 表已创建"

# 3. 创建 MySQL ADS 表
echo ""
echo "[3/4] 创建 MySQL ADS 表..."
cat "$SCRIPT_DIR/../sql/mysql_ads_ddl.sql" | $SSH_CMD \
    "mysql -u hive -pHive@123456" 2>&1 | tail -5
echo "  MySQL ADS 表已创建"

# 4. 验证远端脚本
echo ""
echo "[4/4] 验证远端脚本..."
$SSH_CMD "test -s /home/hadoop/sql/spark_etl.sql && test -x /home/hadoop/scripts/export_to_mysql.py && test -x /home/hadoop/mapreduce/mapper.py && test -x /home/hadoop/mapreduce/reducer.py"
echo "  远端脚本可用"

echo ""
echo "================================================"
echo "  初始化完成"
echo "================================================"
echo ""
echo "  后续日常流程:"
echo "    1) crawler 每天 00:00 自动运行"
echo "    2) bash scripts/run_pipeline.sh  手动触发 ETL"
echo "    3) 或添加到 crawler.service 的 ExecStartPost"
