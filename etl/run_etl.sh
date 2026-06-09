#!/bin/bash
set -e
cd /home/inkpear/etl
export DB_HOST=127.0.0.1 DB_USER=root DB_PASSWORD=root DB_NAME=bilibili
YESTERDAY=
LOG="/home/inkpear/etl/logs/etl_.log"
mkdir -p "."

echo "[Tue Jun  9 22:24:55 CST 2026] Waiting for crawler " | tee -a ""
python3 /home/inkpear/etl/wait_crawler.py "" 2>&1 | tee -a ""

echo "[Tue Jun  9 22:24:55 CST 2026] Running ETL" | tee -a ""
python3 main.py "" --data-dir /home/inkpear/output 2>&1 | tee -a ""
echo "[Tue Jun  9 22:24:55 CST 2026] ETL done" | tee -a ""
