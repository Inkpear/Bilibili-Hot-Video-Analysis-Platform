#!/bin/bash
# run_tag_cloud_mr.sh — 词频 MapReduce → Hive ads_word_cloud
#
# 用法:
#   SOURCE_TYPE=title bash run_tag_cloud_mr.sh     # 标题词云（默认）
#   SOURCE_TYPE=comment bash run_tag_cloud_mr.sh   # 评论词云
#   SOURCE_TYPE=danmaku bash run_tag_cloud_mr.sh   # 弹幕词云

set -euo pipefail

MASTER_HOST="${MASTER_HOST:-192.168.100.10}"
MASTER_USER="${MASTER_USER:-hadoop}"
MASTER_PASS="${MASTER_PASS:-hadoop123}"
SOURCE_TYPE="${SOURCE_TYPE:-title}"

# 检测是否在本地运行
LOCAL_HOSTNAME=$(hostname -s 2>/dev/null || echo "")
if [ "$LOCAL_HOSTNAME" = "master" ] || [ "$LOCAL_HOSTNAME" = "localhost" ]; then
    RUN_CMD=""
else
    SSH_CMD="sshpass -p ${MASTER_PASS} ssh -o StrictHostKeyChecking=no ${MASTER_USER}@${MASTER_HOST}"
    RUN_CMD="$SSH_CMD"
fi

MR_INPUT="/data/bilibili/mr_input/${SOURCE_TYPE}_words"
MR_OUTPUT="/data/bilibili/mr_output/${SOURCE_TYPE}_words"
MR_HOME="/home/hadoop/mapreduce"
TOP_ALL="${TOP_ALL:-150}"
TOP_CATEGORY="${TOP_CATEGORY:-100}"
MIN_COUNT="${MIN_COUNT:-2}"

case "$SOURCE_TYPE" in
    title)
        LABEL="标题词云"
        STOPWORDS="stopwords.txt"
        JOB_NAME="bilibili-title-wordcount"
        INPUT_SQL="SELECT category, REGEXP_REPLACE(title, '[\t\r\n]+', ' ')
            FROM ads_hot_video_rank
            WHERE title IS NOT NULL AND title != '' AND category IS NOT NULL AND category != '';"
        ;;
    comment)
        LABEL="评论词云"
        STOPWORDS="stopwords_comment.txt"
        JOB_NAME="bilibili-comment-wordcount"
        INPUT_SQL="SELECT h.category, REGEXP_REPLACE(c.content, '[\t\r\n]+', ' ')
            FROM dwd_comment_info c
            JOIN ads_hot_video_rank h ON c.bvid = h.bvid
            WHERE c.content IS NOT NULL AND c.content != '' AND h.category IS NOT NULL AND h.category != '';"
        ;;
    danmaku)
        LABEL="弹幕词云"
        STOPWORDS="stopwords_danmaku.txt"
        JOB_NAME="bilibili-danmaku-wordcount"
        INPUT_SQL="SELECT h.category, REGEXP_REPLACE(d.content, '[\t\r\n]+', ' ')
            FROM dwd_danmaku_info d
            JOIN ads_hot_video_rank h ON d.bvid = h.bvid
            WHERE d.content IS NOT NULL AND d.content != '' AND h.category IS NOT NULL AND h.category != '';"
        ;;
    *)
        echo "错误: 不支持的 source_type=$SOURCE_TYPE，允许值: title/comment/danmaku"
        exit 1
        ;;
esac

echo "================================================"
echo "  ${LABEL} MapReduce — $(date '+%Y-%m-%d %H:%M:%S')"
echo "  source_type=${SOURCE_TYPE}"
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
${INPUT_SQL}
EOSQL

if [ -n "$RUN_CMD" ]; then
    cat /tmp/mr_input_gen.sql | $RUN_CMD "cat > /tmp/mr_input_gen.sql && spark-sql --master spark://master:7077 -f /tmp/mr_input_gen.sql" >/dev/null
else
    spark-sql --master spark://master:7077 -f /tmp/mr_input_gen.sql >/dev/null
fi

echo "[2/4] 执行 Hadoop Streaming..."
if [ -n "$RUN_CMD" ]; then
    $RUN_CMD "cd ${MR_HOME} && hadoop jar /opt/hadoop/share/hadoop/tools/lib/hadoop-streaming-3.3.6.jar \
          -D mapreduce.job.name=${JOB_NAME} \
          -D stream.num.map.output.key.fields=2 \
          -file mapper.py \
          -file reducer.py \
          -file ${STOPWORDS} \
          -mapper 'python3 mapper.py ${STOPWORDS}' \
          -reducer 'python3 reducer.py' \
          -input ${MR_INPUT} \
          -output ${MR_OUTPUT}"
else
    cd ${MR_HOME} && hadoop jar /opt/hadoop/share/hadoop/tools/lib/hadoop-streaming-3.3.6.jar \
          -D mapreduce.job.name=${JOB_NAME} \
          -D stream.num.map.output.key.fields=2 \
          -file mapper.py \
          -file reducer.py \
          -file ${STOPWORDS} \
          -mapper "python3 mapper.py ${STOPWORDS}" \
          -reducer 'python3 reducer.py' \
          -input ${MR_INPUT} \
          -output ${MR_OUTPUT}
fi

echo "[3/4] 导入 Hive (source_type=${SOURCE_TYPE})..."
cat > /tmp/mr_import.sql << EOSQL
USE bilibili;
DROP TABLE IF EXISTS mr_${SOURCE_TYPE}_raw;
CREATE EXTERNAL TABLE mr_${SOURCE_TYPE}_raw (
  category STRING,
  word STRING,
  word_count INT
)
ROW FORMAT DELIMITED FIELDS TERMINATED BY '\t'
LOCATION '${MR_OUTPUT}';
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
FROM mr_${SOURCE_TYPE}_raw
WHERE word IS NOT NULL AND word != '' AND word_count >= ${MIN_COUNT}
GROUP BY category
ORDER BY category;
EOSQL

if [ -n "$RUN_CMD" ]; then
    cat /tmp/mr_verify.sql | $RUN_CMD "cat > /tmp/mr_verify.sql && spark-sql --master spark://master:7077 -f /tmp/mr_verify.sql"
else
    spark-sql --master spark://master:7077 -f /tmp/mr_verify.sql
fi

echo "${LABEL} MapReduce 完成"
