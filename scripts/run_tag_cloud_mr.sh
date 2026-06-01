#!/bin/bash
# run_tag_cloud_mr.sh — 标题词频 MapReduce → Hive ads_tag_cloud

set -euo pipefail

MASTER_HOST="${MASTER_HOST:-192.168.100.10}"
MASTER_USER="${MASTER_USER:-hadoop}"
MASTER_PASS="${MASTER_PASS:-hadoop123}"

# 检测是否在本地运行
LOCAL_HOSTNAME=$(hostname -s 2>/dev/null || echo "")
if [ "$LOCAL_HOSTNAME" = "master" ] || [ "$LOCAL_HOSTNAME" = "localhost" ]; then
    # 本地运行模式
    RUN_CMD=""
else
    # 远程运行模式
    SSH_CMD="sshpass -p ${MASTER_PASS} ssh -o StrictHostKeyChecking=no ${MASTER_USER}@${MASTER_HOST}"
    RUN_CMD="$SSH_CMD"
fi

MR_INPUT="/data/bilibili/mr_input/title_words"
MR_OUTPUT="/data/bilibili/mr_output/title_words"
MR_HOME="/home/hadoop/mapreduce"
TOP_ALL="${TOP_ALL:-150}"
TOP_CATEGORY="${TOP_CATEGORY:-100}"
MIN_COUNT="${MIN_COUNT:-2}"

echo "================================================"
echo "  标题词云 MapReduce — $(date '+%Y-%m-%d %H:%M:%S')"
echo "================================================"

echo "[1/4] 生成 MapReduce 输入..."
if [ -n "$RUN_CMD" ]; then
    $RUN_CMD "hdfs dfs -rm -r -f ${MR_INPUT} ${MR_OUTPUT} >/dev/null 2>&1 || true"
else
    hdfs dfs -rm -r -f ${MR_INPUT} ${MR_OUTPUT} >/dev/null 2>&1 || true
fi

cat > /tmp/mr_input_gen.sql << EOSQL
USE bilibili;
INSERT OVERWRITE DIRECTORY '${MR_INPUT}'
ROW FORMAT DELIMITED FIELDS TERMINATED BY '\t'
SELECT category, REGEXP_REPLACE(title, '[\t\r\n]+', ' ')
FROM ads_hot_video_rank
WHERE title IS NOT NULL AND title != '' AND category IS NOT NULL AND category != '';
EOSQL

if [ -n "$RUN_CMD" ]; then
    cat /tmp/mr_input_gen.sql | $RUN_CMD "cat > /tmp/mr_input_gen.sql && spark-sql --master spark://master:7077 -f /tmp/mr_input_gen.sql" >/dev/null
else
    spark-sql --master spark://master:7077 -f /tmp/mr_input_gen.sql >/dev/null
fi

echo "[2/4] 执行 Hadoop Streaming..."
if [ -n "$RUN_CMD" ]; then
    $RUN_CMD "cd ${MR_HOME} && hadoop jar /opt/hadoop/share/hadoop/tools/lib/hadoop-streaming-3.3.6.jar \
          -D mapreduce.job.name=bilibili-title-wordcount \
          -D stream.num.map.output.key.fields=2 \
          -file mapper.py \
          -file reducer.py \
          -file stopwords.txt \
          -mapper 'python3 mapper.py' \
          -reducer 'python3 reducer.py' \
          -input ${MR_INPUT} \
          -output ${MR_OUTPUT}"
else
    cd ${MR_HOME} && hadoop jar /opt/hadoop/share/hadoop/tools/lib/hadoop-streaming-3.3.6.jar \
          -D mapreduce.job.name=bilibili-title-wordcount \
          -D stream.num.map.output.key.fields=2 \
          -file mapper.py \
          -file reducer.py \
          -file stopwords.txt \
          -mapper 'python3 mapper.py' \
          -reducer 'python3 reducer.py' \
          -input ${MR_INPUT} \
          -output ${MR_OUTPUT}
fi

echo "[3/4] 导入 Hive ads_tag_cloud..."
cat > /tmp/mr_import.sql << EOSQL
USE bilibili;
DROP TABLE IF EXISTS mr_tag_cloud_raw;
CREATE EXTERNAL TABLE mr_tag_cloud_raw (
  category STRING,
  tag STRING,
  tag_count INT
)
ROW FORMAT DELIMITED FIELDS TERMINATED BY '\t'
LOCATION '${MR_OUTPUT}';

DROP TABLE IF EXISTS ads_tag_cloud;
CREATE TABLE ads_tag_cloud (tag STRING, tag_count INT, category STRING) STORED AS PARQUET;
INSERT INTO TABLE ads_tag_cloud
SELECT tag, tag_count,
       CASE WHEN category = '__ALL__' THEN '全站' ELSE category END AS category
FROM (
  SELECT category, tag, tag_count,
         ROW_NUMBER() OVER (
           PARTITION BY category ORDER BY tag_count DESC, tag ASC
         ) AS rn
  FROM mr_tag_cloud_raw
  WHERE tag IS NOT NULL AND tag != '' AND tag_count >= ${MIN_COUNT}
) t
WHERE (category = '__ALL__' AND rn <= ${TOP_ALL})
   OR (category != '__ALL__' AND rn <= ${TOP_CATEGORY});
EOSQL

if [ -n "$RUN_CMD" ]; then
    cat /tmp/mr_import.sql | $RUN_CMD "cat > /tmp/mr_import.sql && spark-sql --master spark://master:7077 -f /tmp/mr_import.sql" >/dev/null
else
    spark-sql --master spark://master:7077 -f /tmp/mr_import.sql >/dev/null
fi

echo "[4/4] 验证词云结果..."
cat > /tmp/mr_verify.sql << EOSQL
USE bilibili;
SELECT category, COUNT(*) AS word_count
FROM ads_tag_cloud
GROUP BY category
ORDER BY category;
EOSQL

if [ -n "$RUN_CMD" ]; then
    cat /tmp/mr_verify.sql | $RUN_CMD "cat > /tmp/mr_verify.sql && spark-sql --master spark://master:7077 -f /tmp/mr_verify.sql"
else
    spark-sql --master spark://master:7077 -f /tmp/mr_verify.sql
fi

echo "词云 MapReduce 完成"
