#!/bin/bash
# run_tag_cloud_mr.sh — 标题词频 MapReduce → Hive ads_tag_cloud

set -euo pipefail

MASTER_HOST="${MASTER_HOST:-192.168.100.10}"
MASTER_USER="${MASTER_USER:-hadoop}"
MASTER_PASS="${MASTER_PASS:-hadoop123}"
SSH_CMD="sshpass -p ${MASTER_PASS} ssh -o StrictHostKeyChecking=no ${MASTER_USER}@${MASTER_HOST}"

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
$SSH_CMD "hdfs dfs -rm -r -f ${MR_INPUT} ${MR_OUTPUT} >/dev/null 2>&1 || true"
$SSH_CMD "spark-sql --master spark://master:7077 -e \"
USE bilibili;
INSERT OVERWRITE DIRECTORY '${MR_INPUT}'
ROW FORMAT DELIMITED FIELDS TERMINATED BY '\\t'
SELECT category, REGEXP_REPLACE(title, '[\\t\\r\\n]+', ' ')
FROM ads_hot_video_rank
WHERE title IS NOT NULL AND title != '' AND category IS NOT NULL AND category != '';
\"" >/dev/null

echo "[2/4] 执行 Hadoop Streaming..."
$SSH_CMD "cd ${MR_HOME} && STREAMING_JAR=\$(for jar in /opt/hadoop/share/hadoop/tools/lib/hadoop-streaming-*.jar; do printf '%s' \"\$jar\"; break; done) && \
    hadoop jar \"\$STREAMING_JAR\" \
      -D mapreduce.job.name=bilibili-title-wordcount \
      -D yarn.app.mapreduce.am.env=HADOOP_MAPRED_HOME=/opt/hadoop \
      -D mapreduce.map.env=HADOOP_MAPRED_HOME=/opt/hadoop \
      -D mapreduce.reduce.env=HADOOP_MAPRED_HOME=/opt/hadoop \
      -D stream.num.map.output.key.fields=2 \
      -D mapreduce.partition.keypartitioner.options=-k1,1 \
      -partitioner org.apache.hadoop.mapred.lib.KeyFieldBasedPartitioner \
      -files mapper.py,reducer.py,stopwords.txt \
      -mapper 'python3 mapper.py' \
      -reducer 'python3 reducer.py' \
      -input ${MR_INPUT} \
      -output ${MR_OUTPUT}"

echo "[3/4] 导入 Hive ads_tag_cloud..."
$SSH_CMD "spark-sql --master spark://master:7077 -e \"
USE bilibili;
DROP TABLE IF EXISTS mr_tag_cloud_raw;
CREATE EXTERNAL TABLE mr_tag_cloud_raw (
  category STRING,
  tag STRING,
  tag_count INT
)
ROW FORMAT DELIMITED FIELDS TERMINATED BY '\\t'
LOCATION '${MR_OUTPUT}';

INSERT OVERWRITE TABLE ads_tag_cloud
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

DROP TABLE IF EXISTS mr_tag_cloud_raw;
\"" >/dev/null

echo "[4/4] 验证词云结果..."
$SSH_CMD "spark-sql --master spark://master:7077 -e \"
USE bilibili;
SELECT category, COUNT(*) AS word_count
FROM ads_tag_cloud
GROUP BY category
ORDER BY category;
\""

echo "词云 MapReduce 完成"
