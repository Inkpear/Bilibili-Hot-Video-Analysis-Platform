# AGENTS.md

> 大数据课设项目：B站热门视频 Hadoop 分析与可视化平台

## Overview

爬取 B站热门排行榜 7 天数据，经 HDFS → Hive → Spark SQL 处理后存入 MySQL，Rust Axum 提供只读 API，React + ECharts 展示交互大屏。

```
Python requests 采集(NDJSON) → HDFS → Hive ODS → Spark SQL DWD/DWS/ADS → MySQL → Rust Axum → React
```

## Repo Layout

```
hadoop_lab/
├── docs/plan.md             # 完整开发指南（必读）
├── api-demo/                # B站 API 调用示例，8 个端点均有可运行 Demo
├── crawler/                 # 采集脚本（待实现）
├── sql/                     # Hive/Spark/MySQL DDL 和 ETL SQL（待实现）
├── backend/src/             # Rust Axum 只读 API（待实现）
├── frontend/src/            # React + ECharts 大屏（待实现）
├── scripts/                 # 自动化脚本
├── mapreduce/               # MapReduce 作业（标题词频统计）
├── output/                  # 采集数据
├── report/                  # 课程报告
└── ppt/                     # 答辩 PPT
```

## Key Decisions

| 项 | 决定 |
|---|---|
| 数据来源 | 全站热门 + 10 分区排行 + 每周必看 |
| 采集窗口 | 7 天，每 6 小时 1 次，共 28 快照点 |
| 评论上限 | 200 条/视频（去重后只采一次） |
| 弹幕 | 段式 seg.so API，protobuf 自解析，3000 条/视频（去重后只采一次） |
| 分区 | 动画、番剧、游戏、娱乐、生活、音乐、舞蹈、科技、知识、影视 |
| 输出格式 | NDJSON（每行一个 JSON 对象），含 aid/cid/collect_time/category/snapshot_id |
| 存储 | ODS/DWD/DWS → Hive；ADS → MySQL（Spark JDBC 导出） |
| 前端 | React 18 + TypeScript + ECharts |
| 后端 | Rust Axum，8 个 GET 端点，端口 3000，CORS 全开 |
| 错误码 | 0=成功，400=参数错误，500=内部错误 |

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
- [x] `api-demo/` 8 个 B站 API 全部验证通过、Demo 可运行
- [ ] Hadoop/Hive/Spark 环境搭建
- [ ] 采集脚本开发
- [ ] Hive DDL + Spark SQL ETL
- [ ] MySQL 建表 + 数据导出
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
- 同一视频跨榜单时，ADS 按优先级 canonical：`popular > ranking > weekly`，只保留一条
- tags 来自 `/x/web-interface/view` 的 `data["tags"]`（列表结构，提取 `tag_name` 拼接），`data["dynamic"]` 不是标签
- weekly 必看 API 返回的 stat 字段可能不完整，crawler 需对 weekly 视频补充 `/view` 调用获取完整互动数据
- 前端 TypeScript 类型与 API JSON 一一对应（共享 `docs/plan.md` Section 6.3 定义）
- 指标口径统一用"样本"限定，不使用"全站"
- 归一化方法：`LN(x+1)` 后做 min-max，NULLIF 除零保护 + COALESCE 兜底
- `rank_change` 正数=排名上升（数值变小）
- 资源不存在统一返回 `code=0, data=null`，不使用 404
- Cookie 不硬编码，使用环境变量 `BILI_COOKIE`
