# AGENTS.md

> 大数据课设项目：B站热门视频 Hadoop 分析与可视化平台

## Overview

爬取 B站热门排行榜 7 天数据，经 HDFS → Hive → Spark SQL 处理后存入 MySQL，Rust Axum 提供只读 API，React + ECharts 展示交互大屏。

```
Python requests 采集(NDJSON) → HDFS → Hive ODS → Spark SQL DWD/DWS/ADS → MapReduce 标题词云 → MySQL → Rust Axum → React
```

## Repo Layout

```
Bilibili-Hot-Video-Analysis-Platform/
├── AGENTS.md                # 项目约定与 Agent 指令
├── history.md               # 项目全阶段操作留痕（报告素材）
├── docs/plan.md             # 完整开发指南（必读）
├── api-demo/                # B站 API 调用示例，8 个端点均有可运行 Demo
├── crawler/                 # 采集脚本（已完成）
├── sql/                     # Hive/Spark/MySQL DDL 和 ETL SQL（已完成）
├── backend/src/             # Rust Axum 只读 API（待实现）
├── frontend/src/            # React + ECharts 大屏（待实现）
├── scripts/
│   ├── deploy_crawler.sh    # 一键部署 crawler 到 WSL
│   ├── extract_cookie.py    # 从浏览器提取 Cookie
│   ├── upload_hdfs.sh       # NDJSON → HDFS
│   ├── export_to_mysql.py   # Spark JDBC → MySQL
│   ├── run_pipeline.sh      # 日常 ETL 管线
│   ├── setup_pipeline.sh    # 首次初始化管线
│   └── qemu/                # QEMU/libvirt 虚拟机管理脚本
├── mapreduce/               # MapReduce 作业（标题词频统计）
├── output/                  # 采集数据
├── report/                  # 课程报告
└── ppt/                     # 答辩 PPT
```

## Key Decisions

| 项 | 决定 |
|---|---|
| 数据来源 | 全站热门 + 9 分区排行 |
| 采集窗口 | 7 天，每天 1 轮，共 7 轮 |
| 评论上限 | 200 条/视频（去重后只采一次） |
| 弹幕 | 段式 seg.so API，protobuf 自解析，3000 条/视频（去重后只采一次） |
| 分区 | 动画、游戏、娱乐、生活、音乐、舞蹈、科技、知识、影视 |
| 输出格式 | NDJSON（每行一个 JSON 对象），含 aid/cid/collect_time/category/snapshot_id |
| 存储 | ODS/DWD/DWS → Hive；ADS → MySQL（Spark JDBC 导出） |
| 前端 | React 18 + TypeScript + ECharts |
| 后端 | Rust Axum，8 个 GET 端点，端口 3000，CORS 全开 |
| 错误码 | 0=成功，400=参数错误，500=内部错误 |
| 标签词云 | Hadoop Streaming + Python3/jieba 标题分词，写入 ads_tag_cloud |

## Cluster Architecture

Hadoop 集群运行在 WSL Ubuntu 上的 libvirt 管理的 3 台 CentOS 7 虚拟机中。

| 节点 | IP | 内存 | CPU | 角色 |
|---|---|---|---|---|
| master | 192.168.100.10 | 4G | 2 | NameNode, DataNode, ResourceManager, NodeManager, Hive Metastore, HiveServer2, Spark Master, Spark Worker, Spark History Server, MySQL 8 |
| worker1 | 192.168.100.11 | 2G | 2 | DataNode, NodeManager, Spark Worker |
| worker2 | 192.168.100.12 | 2G | 2 | DataNode, NodeManager, Spark Worker |

**连接方式**：
```bash
ssh UbuntuDesktop                     # 进入 WSL（通过反向隧道）
ssh hadoop@192.168.100.10             # master
ssh hadoop@192.168.100.11             # worker1
ssh hadoop@192.168.100.12             # worker2
# 密码: hadoop123
```

**数据库密码**：
| 服务 | 用户 | 密码 |
|---|---|---|
| MySQL root | root | Root@123456 |
| MySQL hive | hive | Hive@123456 |
| Hive Metastore | hive | Hive@123456 |

**集群管理**：
```bash
# HDFS
hdfs dfsadmin -report
hdfs dfs -ls /

# YARN
yarn node -list
yarn application -list

# Hive
hive -e "SHOW DATABASES;"
beeline -u "jdbc:hive2://master:10000"

# Spark
spark-shell --master spark://master:7077
spark-sql --master spark://master:7077

# MapReduce 词云依赖（三节点已安装）
python3 --version        # Python 3.6.8
python3 -c "import jieba; print(jieba.__version__)"  # 0.42.1

# 启停
start-dfs.sh && start-yarn.sh
stop-yarn.sh && stop-dfs.sh
hive --service metastore &
hive --service hiveserver2 &
/opt/spark/sbin/start-all.sh
/opt/spark/sbin/start-history-server.sh
/opt/spark/sbin/stop-all.sh
```

**libvirt 管理**：
```bash
virsh list --all
virsh start master
virsh shutdown master
virsh net-dhcp-leases hadoop-net
```

**网络**：libvirt 自定义 NAT 网络 `hadoop-net` (192.168.100.0/24)，DHCP 静态绑定。

**详细搭建记录**：`history.md`

## API Contract

8 个只读端点，统一响应 `{"code":0,"message":"ok","data":{...}}`：

| 端点 | 用途 |
|---|---|
| `GET /api/overview` | 大屏总览 |
| `GET /api/hot-videos?sort=heat&limit=20&offset=0` | 热门排行（分页） |
| `GET /api/partition-compare` | 分区竞争力 |
| `GET /api/lifecycle?bvid=BVxxx` | 单视频生命周期 |
| `GET /api/lifecycle/top?limit=5` | TOP N 生命周期 |
| `GET /api/partition-detail?category=游戏` | 分区下钻 |
| `GET /api/tag-cloud?category=游戏` | 标签词云 |
| `GET /api/interaction?bvid=BVxxx` | 互动质量 |

完整 JSON 示例和 TypeScript 类型见 `docs/plan.md` Section 5.3 / 6.3。

## Current State

- [x] `docs/plan.md` 完成
- [x] `api-demo/` 6 个 B站 API 验证通过、Demo 可运行
- [x] Hadoop/Hive/Spark 环境搭建（Hadoop 3.3.6 + Hive 3.1.3 + Spark 3.5.3 集群已启动）
- [x] 采集脚本开发 — 事务模型：日结模式，每视频按 /view→UP→评论→弹幕 全量采集，fail_count≥3 永久放弃
- [x] Hive DDL + Spark SQL ETL — ODS/DWD/DWS/ADS 全链路 SQL，已部署到集群
- [x] MySQL 建表 + 数据导出 — 7 张 ADS 表，Spark JDBC 导出脚本
- [x] MapReduce 标题词云 — Hadoop Streaming + Python3/jieba，已接入日常管线
- [x] 数据管线全链路验证 — P10 问题已解决，7 张 ADS 表全部导出 MySQL 成功
- [ ] Rust 后端 8 个 API 实现
- [ ] React 前端大屏开发
- [ ] 课程报告 + PPT

## Conventions

- 所有字段用 `snake_case`
- 采集输出为 NDJSON（每行一个 JSON 对象），不使用 JSON 数组
- 每轮采集共用一个 `snapshot_id`（格式 `yyyyMMdd_HHmm`），ADS 用 `MAX(snapshot_id)` 取最新快照
- Hive 快照表按 `snapshot_date`（yyyyMMdd）分区，生命周期按 `snapshot_id` 排序
- 时间格式统一 `yyyy-MM-dd HH:mm:ss`
- 视频基础信息按 `bvid` 唯一；快照按 `bvid + snapshot_id + rank_source` 去重
- 同一视频跨榜单时，ADS 按优先级 canonical：`popular > ranking`，只保留一条
- tags 来自 `/x/web-interface/view` 的 `data["tags"]`（列表结构，提取 `tag_name` 拼接），`data["dynamic"]` 不是标签
- 前端 TypeScript 类型与 API JSON 一一对应（共享 `docs/plan.md` Section 6.3 定义）
- 指标口径统一用"样本"限定，不使用"全站"
- 归一化方法：`LN(x+1)` 后做 min-max，NULLIF 除零保护 + COALESCE 兜底
- `rank_change` 正数=排名上升（数值变小）
- 资源不存在统一返回 `code=0, data=null`，不使用 404
- Cookie 不硬编码，使用环境变量 `BILI_COOKIE`
- Cookie 含分号时不能用 `export $(grep ... | xargs)` 提取（bash 会将 `;` 解析为命令分隔符），必须用 `grep -o | sed` 提取原始值
- 采集部署在 WSL，通过 user-level systemd 常驻 `crawler.service` 自动循环触发；日志用 `journalctl --user -u crawler.service -f` 查看
- `crawler.service` 启动即采集，采集完成后自动运行 `run_pipeline.sh`，再休眠 6 小时进入下一轮；需要停止时手动 `systemctl --user stop crawler.service`
- 热门视频 tags 全空（B站数据现象），标签词云由 `scripts/run_tag_cloud_mr.sh` 基于 `ads_hot_video_rank.title` 做 MapReduce 标题分词生成
- 生命周期折线图 Day 1 仅有 1 个快照点（`view_growth=0`），至少需要 2 轮采集
