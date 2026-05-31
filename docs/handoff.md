# 项目交接文档

更新时间：2026-05-31

## 当前阶段

当前分支：`crawler/core`

本阶段已完成：
- Python 爬虫事务模型：全站热门 + 9 分区排行，每视频按详情、UP、评论、弹幕采集。
- HDFS / Hive ODS / Spark DWD-DWS-ADS / MySQL ADS 全链路脚本。
- Hadoop Streaming 标题词云：`mapreduce/mapper.py` + `reducer.py` + `jieba`。
- WSL user-level systemd 常驻采集服务：启动即采集，采完跑 ETL，休眠 6 小时再采。

尚未完成：
- Rust Axum 后端 8 个 API。
- React + ECharts 前端大屏。
- 课程报告和答辩 PPT。

## 当前运行状态

WSL 上常驻服务：
```bash
ssh UbuntuDesktop
systemctl --user status crawler.service
journalctl --user -u crawler.service -f
```

当前机制：
```text
scripts/crawler_loop.sh
→ python3 ~/crawler/main.py --once
→ bash ~/scripts/run_pipeline.sh
→ sleep 21600
→ 下一轮
```

停止采集：
```bash
ssh UbuntuDesktop
systemctl --user stop crawler.service
```

禁用守护进程：
```bash
ssh UbuntuDesktop
systemctl --user disable crawler.service
```

## 集群信息

连接：
```bash
ssh UbuntuDesktop
ssh hadoop@192.168.100.10  # master
ssh hadoop@192.168.100.11  # worker1
ssh hadoop@192.168.100.12  # worker2
```

密码：`hadoop123`

MySQL：
```text
host: master:3306
database: bilibili
user: hive
password: Hive@123456
```

三节点已安装：
```bash
python3 --version  # 3.6.8
python3 -c "import jieba; print(jieba.__version__)"  # 0.42.1
```

## 自动化管线

日常管线入口：
```bash
ssh UbuntuDesktop
bash ~/scripts/run_pipeline.sh
```

步骤：
```text
1. upload_hdfs.sh：NDJSON 上传 HDFS
2. spark-sql：ODS → DWD → DWS → ADS
3. run_tag_cloud_mr.sh：标题词云 MapReduce → ads_tag_cloud
4. export_to_mysql.py：Hive ADS → MySQL ADS
```

首次初始化或重新部署远端脚本：
```bash
ssh UbuntuDesktop
bash ~/scripts/setup_pipeline.sh
```

## 数据检查命令

MySQL ADS 行数：
```bash
ssh UbuntuDesktop
sshpass -p hadoop123 ssh hadoop@192.168.100.10 \
  'mysql -u hive -pHive@123456 bilibili -e "SHOW TABLES; SELECT COUNT(*) FROM ads_hot_video_rank; SELECT COUNT(*) FROM ads_tag_cloud;"'
```

HDFS ODS：
```bash
ssh UbuntuDesktop
sshpass -p hadoop123 ssh hadoop@192.168.100.10 \
  'hdfs dfs -ls -R /data/bilibili/ods | tail -30'
```

Hive/Spark：
```bash
ssh UbuntuDesktop
sshpass -p hadoop123 ssh hadoop@192.168.100.10 \
  'spark-sql --master spark://master:7077 -e "USE bilibili; SHOW TABLES;"'
```

## 下一阶段建议

可以立即开始 Rust Axum 后端骨架：
- 建立 `backend/Cargo.toml` 和 `backend/src/*`。
- 实现 MySQL 连接池。
- 实现统一响应结构 `ApiResponse<T>`。
- 注册 8 个 GET 端点。
- 先兼容空表返回，再等第一轮 ETL 完成后联调真实数据。

真实数据联调建议等待：
```text
journalctl --user -u crawler.service -f
出现：
[crawler-loop] ETL 完成
[crawler-loop] 休眠 21600s
```

## 注意事项

- 不要在 crawler 正在运行时手动删除 `crawler_state.db*`。
- 当前部署脚本已排除 `crawler_state.db*` 和 `logs/`，后续部署不会破坏 SQLite WAL。
- `rank_video.ndjson` 是每轮视频处理完成后才写出，采集中途不存在是正常现象。
- 第一轮生命周期只有一个点，第二轮后才适合展示趋势。
- tags 原始字段为空是 B站数据现象，词云来自标题 MapReduce，不来自 tags。

## 合并后清理分支

PR 合并后删除远端开发分支：
```bash
git push origin --delete crawler/core
```

本地删除开发分支：
```bash
git checkout main
git pull
git branch -d crawler/core
```
