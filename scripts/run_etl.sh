#!/bin/bash
set -e
cd /home/inkpear/etl

export DB_HOST=127.0.0.1
export DB_USER=root
export DB_PASSWORD=root
export DB_NAME=bilibili

YESTERDAY=$(date -d 'yesterday' +%Y%m%d)
LOGDIR="/home/inkpear/etl/logs"
mkdir -p "$LOGDIR"
LOG="$LOGDIR/etl_${YESTERDAY}.log"

echo "[$(date)] Waiting for crawler full crawl on ${YESTERDAY}" | tee -a "$LOG"
python3 /home/inkpear/etl/wait_crawler.py "$YESTERDAY" 2>&1 | tee -a "$LOG"

echo "[$(date)] Running ETL for ${YESTERDAY}" | tee -a "$LOG"
python3 main.py "$YESTERDAY" --data-dir /home/inkpear/output 2>&1 | tee -a "$LOG"
echo "[$(date)] ETL done for ${YESTERDAY}" | tee -a "$LOG"
