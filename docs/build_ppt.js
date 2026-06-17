const pptxgen = require("pptxgenjs");
const React = require("react");
const ReactDOMServer = require("react-dom/server");
const sharp = require("sharp");
const fs = require("fs");
const path = require("path");

// ─── Icon helpers ───
const {
  FaRocket, FaServer, FaDownload, FaProjectDiagram,
  FaCubes, FaChartLine, FaChartBar, FaTrophy,
  FaLightbulb, FaCheckCircle, FaDatabase, FaCogs,
  FaDesktop, FaSearch, FaLayerGroup, FaBrain,
} = require("react-icons/fa");

function renderIconSvg(IconComponent, color = "#000000", size = 256) {
  return ReactDOMServer.renderToStaticMarkup(
    React.createElement(IconComponent, { color, size: String(size) })
  );
}

async function iconToBase64(IconComponent, color, size = 256) {
  const svg = renderIconSvg(IconComponent, color, size);
  const pngBuffer = await sharp(Buffer.from(svg)).png().toBuffer();
  return "image/png;base64," + pngBuffer.toString("base64");
}

// ─── Color Palette (Bilibili-inspired) ───
const C = {
  darkBg:    "0F172A",
  darkCard:  "334155",
  lightBg:   "F8FAFC",
  white:     "FFFFFF",
  pink:      "E5668A",
  blue:      "2EA9DF",
  darkText:  "1E293B",
  muted:     "64748B",
  lightText: "CBD5E1",
  divider:   "E2E8F0",
  cardBg:    "F1F5F9",
  navy:      "334155",
};

// ─── Reusable factories (never reuse option objects) ───
const mkShadow = () => ({ type: "outer", blur: 4, offset: 2, angle: 135, color: "000000", opacity: 0.08 });
const mkCardShadow = () => ({ type: "outer", blur: 6, offset: 2, angle: 135, color: "000000", opacity: 0.10 });

// ─── Slide title helper (light slides) ───
function addSlideTitle(slide, title, subtitle) {
  // Colored dot before title
  slide.addShape(slide._slideLayout ? "rect" : "rect", {});
  // Small header bar at top
  slide.addShape("rect", {
    x: 0, y: 0, w: 10, h: 0.08,
    fill: { color: C.blue },
  });
  slide.addText(title, {
    x: 0.7, y: 0.25, w: 8.6, h: 0.6,
    fontSize: 30, fontFace: "Arial",
    color: C.darkText, bold: true, margin: 0,
  });
  if (subtitle) {
    slide.addText(subtitle, {
      x: 0.7, y: 0.75, w: 8.6, h: 0.35,
      fontSize: 13, fontFace: "Arial",
      color: C.muted, margin: 0,
    });
  }
}

// ─── Build ───
async function build() {
  const pres = new pptxgen();
  pres.layout = "LAYOUT_16x9";
  pres.author = "Group 7";
  pres.title = "B站热门视频分析平台 - 答辩PPT";

  // Pre-render icons
  const icons = {};
  const iconList = [
    ["rocket", FaRocket, C.pink],
    ["server", FaServer, C.blue],
    ["download", FaDownload, C.blue],
    ["diagram", FaProjectDiagram, C.blue],
    ["cubes", FaCubes, C.blue],
    ["chartLine", FaChartLine, C.pink],
    ["chartBar", FaChartBar, C.blue],
    ["trophy", FaTrophy, C.pink],
    ["lightbulb", FaLightbulb, C.pink],
    ["check", FaCheckCircle, C.blue],
    ["database", FaDatabase, C.blue],
    ["cogs", FaCogs, C.blue],
    ["desktop", FaDesktop, C.blue],
    ["search", FaSearch, C.pink],
    ["layer", FaLayerGroup, C.blue],
    ["brain", FaBrain, C.pink],
    // White variants for dark slides
    ["rocketW", FaRocket, "#FFFFFF"],
    ["serverW", FaServer, "#FFFFFF"],
    ["chartLineW", FaChartLine, "#FFFFFF"],
    ["trophyW", FaTrophy, "#FFFFFF"],
    ["checkW", FaCheckCircle, "#FFFFFF"],
    ["databaseW", FaDatabase, "#FFFFFF"],
    ["cogsW", FaCogs, "#FFFFFF"],
  ];
  for (const [name, component, color] of iconList) {
    icons[name] = await iconToBase64(component, color);
  }

  const DIAGRAMS = "/Volumes/code/GIT/Bilibili-Hot-Video-Analysis-Platform/docs/diagrams_ppt";

  // ═══════════════════════════════════════
  // SLIDE 1: TITLE (Dark)
  // ═══════════════════════════════════════
  {
    const s = pres.addSlide();
    s.background = { color: C.darkBg };

    // Decorative shapes
    s.addShape("rect", { x: 0, y: 0, w: 10, h: 0.06, fill: { color: C.pink } });
    // Large decorative circle (subtle)
    s.addShape("oval", {
      x: 7.2, y: 2.5, w: 4.5, h: 4.5,
      fill: { color: C.blue, transparency: 93 },
    });
    s.addShape("oval", {
      x: -1.5, y: -0.5, w: 3.5, h: 3.5,
      fill: { color: C.pink, transparency: 93 },
    });

    // Main title
    s.addText("B站热门视频分析平台", {
      x: 0.8, y: 1.5, w: 8.4, h: 1.0,
      fontSize: 42, fontFace: "Arial",
      color: C.white, bold: true, margin: 0,
    });
    // Subtitle
    s.addText("大数据智能化综合实训 · 课程答辩", {
      x: 0.8, y: 2.5, w: 8.4, h: 0.5,
      fontSize: 20, fontFace: "Arial",
      color: C.lightText, margin: 0,
    });

    // Separator
    s.addShape("rect", {
      x: 0.8, y: 3.25, w: 2.0, h: 0.04,
      fill: { color: C.pink },
    });

    // Info
    s.addText([
      { text: "第7组", options: { breakLine: true } },
      { text: "2023级计科1班", options: { breakLine: true } },
      { text: "指导教师：刘友缘  |  2026年6月", options: {} },
    ], {
      x: 0.8, y: 3.55, w: 8.4, h: 1.0,
      fontSize: 14, fontFace: "Arial",
      color: C.muted, lineSpacingMultiple: 1.6, margin: 0,
    });
  }

  // ═══════════════════════════════════════
  // SLIDE 2: 目录 (Light)
  // ═══════════════════════════════════════
  {
    const s = pres.addSlide();
    s.background = { color: C.lightBg };
    addSlideTitle(s, "答辩提纲", "CONTENTS");

    const items = [
      ["01", "项目背景与目标", icons.lightbulb],
      ["02", "系统架构设计", icons.diagram],
      ["03", "数据采集系统", icons.download],
      ["04", "大数据平台搭建", icons.server],
      ["05", "数据仓库设计", icons.cubes],
      ["06", "数据分析与可视化", icons.chartBar],
      ["07", "关键技术亮点", icons.rocket],
      ["08", "项目成果总结", icons.trophy],
    ];

    const startY = 1.3;
    const rowH = 0.95;
    items.forEach((item, i) => {
      const col = i < 4 ? 0 : 1;
      const row = i < 4 ? i : i - 4;
      const x = 0.7 + col * 4.5;
      const y = startY + row * rowH;

      // Card background
      s.addShape("rect", {
        x, y, w: 4.1, h: 0.8,
        fill: { color: C.white },
        shadow: mkCardShadow(),
      });
      // Number
      s.addText(item[0], {
        x: x + 0.15, y: y + 0.1, w: 0.55, h: 0.6,
        fontSize: 22, fontFace: "Arial",
        color: C.pink, bold: true, align: "center", valign: "middle", margin: 0,
      });
      // Icon
      s.addImage({ data: item[2], x: x + 0.7, y: y + 0.15, w: 0.5, h: 0.5 });
      // Title
      s.addText(item[1], {
        x: x + 1.35, y, w: 2.6, h: 0.8,
        fontSize: 15, fontFace: "Arial",
        color: C.darkText, valign: "middle", margin: 0,
      });
    });
  }

  // ═══════════════════════════════════════
  // SLIDE 3: 项目背景与目标 (Light)
  // ═══════════════════════════════════════
  {
    const s = pres.addSlide();
    s.background = { color: C.lightBg };
    addSlideTitle(s, "项目背景与目标", "BACKGROUND & OBJECTIVES");

    // Left: Background
    s.addShape("rect", {
      x: 0.7, y: 1.2, w: 4.1, h: 3.8,
      fill: { color: C.white },
      shadow: mkCardShadow(),
    });
    s.addImage({ data: icons.search, x: 0.9, y: 1.35, w: 0.4, h: 0.4 });
    s.addText("项目背景", {
      x: 1.45, y: 1.3, w: 3, h: 0.5,
      fontSize: 18, fontFace: "Arial",
      color: C.darkText, bold: true, margin: 0,
    });
    s.addText([
      { text: "Bilibili（哔哩哔哩）是中国最大的弹幕视频网站，", options: { breakLine: true } },
      { text: "月活跃用户超过3亿。每天有数以万计的创作者", options: { breakLine: true } },
      { text: "上传视频，各分区热门排行榜动态变化频繁。", options: { breakLine: true } },
      { text: "", options: { breakLine: true, fontSize: 8 } },
      { text: "对大热门视频进行系统性的数据采集、存储、", options: { breakLine: true } },
      { text: "分析和可视化展示，可以帮助内容创作者了解", options: { breakLine: true } },
      { text: "平台热门趋势、优化创作方向，也为普通用户", options: { breakLine: true } },
      { text: "提供更有价值的内容发现工具。", options: {} },
    ], {
      x: 0.9, y: 1.85, w: 3.7, h: 3.0,
      fontSize: 13, fontFace: "Arial",
      color: C.navy, lineSpacingMultiple: 1.5, valign: "top", margin: 0,
    });

    // Right: 4 objectives
    const objectives = [
      ["自动化采集", "Rust高性能爬虫\n14个分区 · 每日全量", icons.download],
      ["大数据平台", "Hadoop HDFS + Hive\n分布式存储与数仓", icons.server],
      ["可视化大屏", "React + ECharts\n交互式数据看板", icons.desktop],
      ["文本分析", "jieba-rs中文分词\n词云与词频趋势", icons.brain],
    ];

    objectives.forEach((obj, i) => {
      const x = 5.2;
      const y = 1.2 + i * 1.0;
      s.addShape("rect", {
        x, y, w: 4.1, h: 0.85,
        fill: { color: C.white },
        shadow: mkCardShadow(),
      });
      // Icon in circle
      s.addShape("oval", {
        x: x + 0.15, y: y + 0.17, w: 0.5, h: 0.5,
        fill: { color: i % 2 === 0 ? C.blue : C.pink, transparency: 15 },
      });
      s.addImage({ data: obj[2], x: x + 0.22, y: y + 0.24, w: 0.36, h: 0.36 });
      s.addText(obj[0], {
        x: x + 0.8, y: y + 0.05, w: 3.1, h: 0.35,
        fontSize: 15, fontFace: "Arial",
        color: C.darkText, bold: true, margin: 0,
      });
      s.addText(obj[1], {
        x: x + 0.8, y: y + 0.38, w: 3.1, h: 0.4,
        fontSize: 11, fontFace: "Arial",
        color: C.muted, margin: 0, lineSpacingMultiple: 1.3,
      });
    });
  }

  // ═══════════════════════════════════════
  // SLIDE 4: 系统架构 (Light)
  // ═══════════════════════════════════════
  {
    const s = pres.addSlide();
    s.background = { color: C.lightBg };
    addSlideTitle(s, "系统整体架构", "SYSTEM ARCHITECTURE");

    // Left: Architecture diagram
    s.addImage({
      path: path.join(DIAGRAMS, "arch_architecture.png"),
      x: 0.5, y: 1.1, w: 3.8, h: 4.3,
      sizing: { type: "contain", w: 3.8, h: 4.3 },
    });

    // Right: Layer descriptions
    const layers = [
      { title: "数据采集层", desc: "Rust 异步爬虫 · WBI v3 签名\n3 账号并发轮换 · tokio 运行时", color: C.pink },
      { title: "数据存储层", desc: "Hadoop HDFS 3.1.3 · 3 节点集群\nDocker Compose 编排 · NDJSON 分区", color: C.blue },
      { title: "数据处理层", desc: "Hive 3.1.2 · Spark 2.2.0\njieba-rs 中文分词 · MySQL Metastore", color: C.pink },
      { title: "数据服务层", desc: "MySQL 8.0 · Rust Axum API\n13 个 REST 端点 · Bearer 鉴权", color: C.blue },
      { title: "数据展示层", desc: "React 19 + ECharts 5 + Vite\nNginx 反向代理 · FRP 公网穿透", color: C.pink },
    ];
    layers.forEach((l, i) => {
      const y = 1.1 + i * 0.85;
      s.addShape("rect", {
        x: 4.6, y, w: 4.7, h: 0.73,
        fill: { color: C.white },
        shadow: mkShadow(),
      });
      s.addShape("rect", {
        x: 4.6, y, w: 0.06, h: 0.73,
        fill: { color: l.color },
      });
      s.addText(l.title, {
        x: 4.85, y: y + 0.02, w: 4.3, h: 0.3,
        fontSize: 13, fontFace: "Arial", color: C.darkText, bold: true, margin: 0,
      });
      s.addText(l.desc, {
        x: 4.85, y: y + 0.32, w: 4.3, h: 0.38,
        fontSize: 10, fontFace: "Arial", color: C.muted, margin: 0, lineSpacingMultiple: 1.2,
      });
    });

    // Bottom summary
    s.addText("五层架构 · 松耦合高内聚  |  NDJSON → HDFS → Hive → MySQL → API → React", {
      x: 0.5, y: 5.15, w: 9.0, h: 0.3,
      fontSize: 10, fontFace: "Arial", color: C.muted,
      align: "center", margin: 0,
    });
  }

  // ═══════════════════════════════════════
  // SLIDE 5: 数据采集系统 (Light)
  // ═══════════════════════════════════════
  {
    const s = pres.addSlide();
    s.background = { color: C.lightBg };
    addSlideTitle(s, "数据采集系统", "DATA CRAWLER");

    // Left: Crawler diagram
    const imgW = 3.7, imgH = 3.6;
    s.addImage({
      path: path.join(DIAGRAMS, "arch_crawler.png"),
      x: 0.7, y: 1.2, w: imgW, h: imgH,
      sizing: { type: "contain", w: imgW, h: imgH },
    });
    // Border around image area
    s.addShape("rect", {
      x: 0.7, y: 1.2, w: imgW, h: imgH,
      fill: { color: C.white, transparency: 100 },
      line: { color: C.divider, width: 0.5 },
    });

    // Right: Key features
    const features = [
      ["Rust + Tokio 异步", "高性能并发爬虫，3账号轮换采集", icons.cogs],
      ["14个内容分区", "影视、游戏、知识、音乐等全覆盖", icons.layer],
      ["WBI v3 签名认证", "反反爬：设备指纹 + 多账号 + 限速", icons.check],
      ["SQLite 状态机", "断点续爬 + 失败重试（最多10次）", icons.database],
      ["NDJSON 输出", "每日原始数据 ~250MB，按日期组织", icons.server],
    ];

    features.forEach((feat, i) => {
      const y = 1.2 + i * 0.75;
      s.addShape("rect", {
        x: 4.7, y, w: 4.6, h: 0.62,
        fill: { color: C.white },
        shadow: mkShadow(),
      });
      // Left accent
      s.addShape("rect", {
        x: 4.7, y, w: 0.06, h: 0.62,
        fill: { color: i % 2 === 0 ? C.blue : C.pink },
      });
      s.addImage({ data: feat[2], x: 4.95, y: y + 0.11, w: 0.38, h: 0.38 });
      s.addText(feat[0], {
        x: 5.45, y: y + 0.02, w: 3.7, h: 0.3,
        fontSize: 13, fontFace: "Arial", color: C.darkText, bold: true, margin: 0,
      });
      s.addText(feat[1], {
        x: 5.45, y: y + 0.32, w: 3.7, h: 0.25,
        fontSize: 10, fontFace: "Arial", color: C.muted, margin: 0,
      });
    });

    // Daily stats at bottom
    const dailyStats = [
      ["1,400+", "视频/天", C.pink],
      ["23万+", "评论/天", C.blue],
      ["85万+", "弹幕/天", C.pink],
      ["1,300+", "UP主/天", C.blue],
    ];
    dailyStats.forEach((ds, i) => {
      const x = 0.7 + i * 2.25;
      s.addShape("rect", {
        x, y: 4.85, w: 2.0, h: 0.45,
        fill: { color: ds[2], transparency: 12 },
      });
      s.addText([
        { text: ds[0] + " ", options: { bold: true, fontSize: 18, color: ds[2] } },
        { text: ds[1], options: { fontSize: 10, color: C.muted } },
      ], {
        x, y: 4.85, w: 2.0, h: 0.45,
        align: "center", valign: "middle", margin: 0, fontFace: "Arial",
      });
    });
  }

  // ═══════════════════════════════════════
  // SLIDE 6: 大数据平台 (Light)
  // ═══════════════════════════════════════
  {
    const s = pres.addSlide();
    s.background = { color: C.lightBg };
    addSlideTitle(s, "大数据平台搭建", "BIG DATA PLATFORM");

    const platforms = [
      {
        title: "Hadoop HDFS 3.1.3",
        desc: "分布式文件系统\n3节点集群（1 NameNode + 2 DataNode）\nDocker Compose 一键编排\n高容错 · 高吞吐",
        icon: icons.database,
        color: C.blue,
      },
      {
        title: "Hive 3.1.2",
        desc: "数据仓库工具\nMySQL Metastore 元数据存储\nHiveServer2 + Beeline 客户端\nHQL → MapReduce/Spark 作业",
        icon: icons.server,
        color: C.pink,
      },
      {
        title: "Spark 2.2.0 (HDP)",
        desc: "分布式计算引擎\nPySpark DataFrame API\nETL 清洗 + 聚合计算\n比 MapReduce 快 100x",
        icon: icons.cogs,
        color: C.blue,
      },
    ];

    platforms.forEach((p, i) => {
      const x = 0.7 + i * 3.0;
      // Card
      s.addShape("rect", {
        x, y: 1.3, w: 2.7, h: 3.6,
        fill: { color: C.white },
        shadow: mkCardShadow(),
      });
      // Top color strip
      s.addShape("rect", {
        x, y: 1.3, w: 2.7, h: 0.06,
        fill: { color: p.color },
      });
      // Icon circle
      s.addShape("oval", {
        x: x + 0.95, y: 1.6, w: 0.8, h: 0.8,
        fill: { color: p.color, transparency: 15 },
      });
      s.addImage({ data: p.icon, x: x + 1.1, y: 1.75, w: 0.5, h: 0.5 });
      // Title
      s.addText(p.title, {
        x: x + 0.2, y: 2.6, w: 2.3, h: 0.5,
        fontSize: 14, fontFace: "Arial", color: C.darkText,
        bold: true, align: "center", margin: 0,
      });
      // Description
      s.addText(p.desc, {
        x: x + 0.25, y: 3.1, w: 2.2, h: 1.6,
        fontSize: 11, fontFace: "Arial", color: C.muted,
        align: "center", lineSpacingMultiple: 1.5, margin: 0,
      });
    });

    // Bottom bar
    s.addShape("rect", {
      x: 0.7, y: 5.1, w: 8.6, h: 0.35,
      fill: { color: C.cardBg },
    });
    s.addText("全部组件通过 Docker 容器化部署在 WSL2 Ubuntu 环境中  |  JDK 8u212  |  ZooKeeper 3.5.7  |  HBase 2.2.2", {
      x: 0.7, y: 5.1, w: 8.6, h: 0.35,
      fontSize: 10, fontFace: "Arial", color: C.muted,
      align: "center", valign: "middle", margin: 0,
    });
  }

  // ═══════════════════════════════════════
  // SLIDE 7: 数据仓库设计 (Light)
  // ═══════════════════════════════════════
  {
    const s = pres.addSlide();
    s.background = { color: C.lightBg };
    addSlideTitle(s, "数据仓库设计", "DATA WAREHOUSE — ODS → DWD → DWS → ADS");

    // Warehouse diagram
    const whW = 4.5, whH = 4.0;
    s.addImage({
      path: path.join(DIAGRAMS, "arch_warehouse.png"),
      x: 0.5, y: 1.1, w: whW, h: whH,
      sizing: { type: "contain", w: whW, h: whH },
    });

    // Right: Layer descriptions
    const layers = [
      {
        title: "ODS 操作数据层",
        desc: "外部表映射 HDFS 原始 NDJSON 文件\n按日期分区存储，JsonSerDe 解析\n保留所有原始数据，不做任何清洗",
        color: C.blue,
      },
      {
        title: "DWD 明细数据层",
        desc: "JSON 字段解析为结构化列\n(bvid, snapshot_date) 组合键去重\n缺失值填充 + 字段类型统一",
        color: C.pink,
      },
      {
        title: "DWS 服务数据层",
        desc: "计算互动率等衍生指标\n按分区 / 日期维度聚合统计\n汇总全站每日总量数据",
        color: C.blue,
      },
      {
        title: "ADS 应用数据层",
        desc: "面向分析场景的结果表\n热度评分排名 · 分区对比 · 词频趋势\n直接对接后端 API 查询",
        color: C.pink,
      },
    ];

    layers.forEach((l, i) => {
      const y = 1.15 + i * 0.95;
      s.addShape("rect", {
        x: 5.0, y, w: 4.3, h: 0.9,
        fill: { color: C.white },
        shadow: mkShadow(),
      });
      s.addShape("rect", {
        x: 5.0, y, w: 0.06, h: 0.9,
        fill: { color: l.color },
      });
      s.addText(l.title, {
        x: 5.25, y: y + 0.05, w: 3.9, h: 0.3,
        fontSize: 14, fontFace: "Arial", color: C.darkText, bold: true, margin: 0,
      });
      s.addText(l.desc, {
        x: 5.25, y: y + 0.35, w: 3.9, h: 0.5,
        fontSize: 10, fontFace: "Arial", color: C.muted, margin: 0, lineSpacingMultiple: 1.3,
      });
    });
  }

  // ═══════════════════════════════════════
  // SLIDE 8: 数据分析模型 (Light)
  // ═══════════════════════════════════════
  {
    const s = pres.addSlide();
    s.background = { color: C.lightBg };
    addSlideTitle(s, "数据分析模型", "ANALYSIS MODELS");

    // Heat Score Formula - prominent display
    s.addShape("rect", {
      x: 0.7, y: 1.2, w: 8.6, h: 1.6,
      fill: { color: C.white },
      shadow: mkCardShadow(),
    });
    s.addShape("rect", {
      x: 0.7, y: 1.2, w: 0.06, h: 1.6,
      fill: { color: C.pink },
    });
    s.addText("热度评分公式", {
      x: 1.0, y: 1.25, w: 8.0, h: 0.35,
      fontSize: 14, fontFace: "Arial", color: C.pink, bold: true, margin: 0,
    });
    s.addText("HeatScore = LN(播放) × 3.0 + LN(点赞) × 3.5 + LN(投币) × 3.0 + LN(收藏) × 4.0 + LN(评论) × 1.0 + LN(分享) × 2.0", {
      x: 1.0, y: 1.65, w: 8.0, h: 0.55,
      fontSize: 12, fontFace: "Arial", color: C.darkText, margin: 0,
    });
    s.addText("收藏权重最高（4.0）→ 表达用户对内容价值的最高认可  |  评论权重最低（1.0）→ 受争议性内容影响较大", {
      x: 1.0, y: 2.2, w: 8.0, h: 0.35,
      fontSize: 10, fontFace: "Arial", color: C.muted, margin: 0,
    });

    // Analysis dimensions
    const dims = [
      ["分区对比分析", "14个分区的平均播放量、互动率、UP主分布横向对比", icons.chartBar],
      ["生命周期追踪", "视频从热门到衰减的播放量/点赞数/排名变化曲线", icons.chartLine],
      ["词云与词频", "jieba-rs 分词 + 679停用词过滤 → 标题/评论/弹幕词云", icons.brain],
      ["互动率雷达", "点赞率 · 投币率 · 收藏率 · 评论率 · 分享率 五维分析", icons.search],
    ];

    dims.forEach((d, i) => {
      const x = 0.7 + (i % 2) * 4.45;
      const y = 3.1 + Math.floor(i / 2) * 1.1;
      s.addShape("rect", {
        x, y, w: 4.15, h: 1.0,
        fill: { color: C.white },
        shadow: mkShadow(),
      });
      // Icon circle
      s.addShape("oval", {
        x: x + 0.15, y: y + 0.17, w: 0.55, h: 0.55,
        fill: { color: i % 2 === 0 ? C.blue : C.pink, transparency: 15 },
      });
      s.addImage({ data: d[2], x: x + 0.22, y: y + 0.24, w: 0.4, h: 0.4 });
      s.addText(d[0], {
        x: x + 0.85, y: y + 0.05, w: 3.1, h: 0.35,
        fontSize: 14, fontFace: "Arial", color: C.darkText, bold: true, margin: 0,
      });
      s.addText(d[1], {
        x: x + 0.85, y: y + 0.42, w: 3.1, h: 0.45,
        fontSize: 10, fontFace: "Arial", color: C.muted, margin: 0,
      });
    });
  }

  // ═══════════════════════════════════════
  // SLIDE 9: 关键技术亮点 (Light)
  // ═══════════════════════════════════════
  {
    const s = pres.addSlide();
    s.background = { color: C.lightBg };
    addSlideTitle(s, "关键技术亮点", "TECHNICAL HIGHLIGHTS");

    const highlights = [
      ["Rust 异步爬虫", "tokio 运行时 · 3账号并发\nWBI v3 签名 · SQLite状态机", icons.cogs],
      ["InnoDB 性能优化", "buffer_pool 4G · log 1G\nETL 时间 2min → 13s (9x)", icons.rocket],
      ["中文分词", "jieba-rs v0.10 · 679 停用词\n标题/评论/弹幕 三维词云", icons.brain],
      ["Docker 全容器化", "Hadoop/Hive/Spark/MySQL\n一键编排 · 环境一致性", icons.server],
      ["四层数仓建模", "ODS→DWD→DWS→ADS\n分区 + 快照设计 · TB级数据", icons.cubes],
      ["前后端分离", "React 19 + Axum + ECharts 5\n13 个 REST API · 交互式大屏", icons.desktop],
    ];

    highlights.forEach((h, i) => {
      const col = i % 3;
      const row = Math.floor(i / 3);
      const x = 0.7 + col * 3.0;
      const y = 1.25 + row * 2.05;

      s.addShape("rect", {
        x, y, w: 2.7, h: 1.85,
        fill: { color: C.white },
        shadow: mkCardShadow(),
      });
      // Top accent
      s.addShape("rect", {
        x, y, w: 2.7, h: 0.06,
        fill: { color: row === 0 ? C.pink : C.blue },
      });

      // Icon
      s.addShape("oval", {
        x: x + 0.9, y: y + 0.25, w: 0.7, h: 0.7,
        fill: { color: row === 0 ? C.pink : C.blue, transparency: 15 },
      });
      s.addImage({ data: h[2], x: x + 1.05, y: y + 0.4, w: 0.4, h: 0.4 });

      // Title
      s.addText(h[0], {
        x: x + 0.15, y: y + 1.05, w: 2.4, h: 0.35,
        fontSize: 14, fontFace: "Arial", color: C.darkText,
        bold: true, align: "center", margin: 0,
      });
      // Desc
      s.addText(h[1], {
        x: x + 0.15, y: y + 1.35, w: 2.4, h: 0.45,
        fontSize: 9.5, fontFace: "Arial", color: C.muted,
        align: "center", lineSpacingMultiple: 1.4, margin: 0,
      });
    });
  }

  // ═══════════════════════════════════════
  // SLIDE 10: 数据可视化 (Light)
  // ═══════════════════════════════════════
  {
    const s = pres.addSlide();
    s.background = { color: C.lightBg };
    addSlideTitle(s, "数据可视化展示", "DATA VISUALIZATION");

    // Frontend diagram
    const fwW = 8.6, fwH = 1.6;
    s.addImage({
      path: path.join(DIAGRAMS, "arch_frontend.png"),
      x: 0.7, y: 1.1, w: fwW, h: fwH,
      sizing: { type: "contain", w: fwW, h: fwH },
    });

    // Tech stack
    s.addText("React 19 + TypeScript + Vite  |  ECharts 5  |  Tailwind CSS  |  Axios  |  Nginx 反向代理", {
      x: 0.7, y: 2.8, w: 8.6, h: 0.35,
      fontSize: 11, fontFace: "Arial", color: C.blue, bold: true,
      align: "center", valign: "middle", margin: 0,
    });

    // Components grid (2x3)
    const components = [
      ["KPI 指标卡", "视频总数 · 总播放量\nUP主数 · 总互动量"],
      ["分区热度柱状图", "14分区多维度对比\n排序/筛选/Hover详情"],
      ["热门视频排行榜", "分页表格 · 多条件排序\n点击联动其他组件"],
      ["生命周期曲线", "播放量/点赞/投币/排名\n多快照追踪 · 插值填充"],
      ["词云图", "标题/评论/弹幕三Tab切换\n前100高频词 · 分区筛选"],
      ["互动率雷达图", "五维互动率对比\n全站最大值参考线"],
    ];

    components.forEach((c, i) => {
      const col = i % 3;
      const row = Math.floor(i / 3);
      const x = 0.7 + col * 3.0;
      const y = 3.2 + row * 1.0;

      s.addShape("rect", {
        x, y, w: 2.7, h: 0.9,
        fill: { color: C.white },
        shadow: mkShadow(),
      });
      s.addShape("rect", {
        x, y, w: 0.05, h: 0.9,
        fill: { color: col === 1 ? C.pink : C.blue },
      });
      s.addText(c[0], {
        x: x + 0.18, y: y + 0.05, w: 2.4, h: 0.3,
        fontSize: 12, fontFace: "Arial", color: C.darkText, bold: true, margin: 0,
      });
      s.addText(c[1], {
        x: x + 0.18, y: y + 0.35, w: 2.4, h: 0.5,
        fontSize: 9, fontFace: "Arial", color: C.muted, margin: 0, lineSpacingMultiple: 1.3,
      });
    });

    // API count
    s.addText("13 个 REST API 端点  |  Rust Axum 框架  |  JSON 响应  |  CORS + Bearer Token 鉴权", {
      x: 0.7, y: 5.05, w: 8.6, h: 0.3,
      fontSize: 10, fontFace: "Arial", color: C.muted,
      align: "center", valign: "middle", margin: 0,
    });
  }

  // ═══════════════════════════════════════
  // SLIDE 11: 项目成果 (Dark)
  // ═══════════════════════════════════════
  {
    const s = pres.addSlide();
    s.background = { color: C.darkBg };

    // Title
    s.addText("项目成果", {
      x: 0.8, y: 0.4, w: 8.4, h: 0.6,
      fontSize: 30, fontFace: "Arial", color: C.white, bold: true, margin: 0,
    });

    // Big numbers
    const metrics = [
      ["1,400+", "日采集视频数", icons.rocketW],
      ["108万+", "日采集总记录数", icons.databaseW],
      ["~250MB", "日原始数据量", icons.serverW],
      ["13s", "ETL处理耗时\n(优化后)", icons.chartLineW],
      ["13", "REST API\n端点", icons.cogsW],
      ["9", "MySQL\n数据表", icons.checkW],
    ];

    metrics.forEach((m, i) => {
      const col = i % 3;
      const row = Math.floor(i / 3);
      const x = 0.7 + col * 3.0;
      const y = 1.3 + row * 2.0;

      s.addShape("rect", {
        x, y, w: 2.7, h: 1.75,
        fill: { color: C.darkCard },
        line: { color: "475569", width: 0.5 },
      });
      s.addShape("rect", {
        x, y, w: 2.7, h: 0.05,
        fill: { color: i % 3 === 0 ? C.pink : i % 3 === 1 ? C.blue : C.pink },
      });

      // Icon
      s.addImage({ data: m[2], x: x + 1.05, y: y + 0.2, w: 0.45, h: 0.45 });

      // Big number
      s.addText(m[0], {
        x: x + 0.1, y: y + 0.7, w: 2.5, h: 0.55,
        fontSize: 28, fontFace: "Arial",
        color: i % 3 === 0 ? C.pink : i % 3 === 1 ? C.blue : C.pink,
        bold: true, align: "center", margin: 0,
      });
      // Label
      s.addText(m[1], {
        x: x + 0.1, y: y + 1.2, w: 2.5, h: 0.45,
        fontSize: 11, fontFace: "Arial", color: C.lightText,
        align: "center", margin: 0, lineSpacingMultiple: 1.2,
      });
    });
  }

  // ═══════════════════════════════════════
  // SLIDE 12: 致谢 (Dark)
  // ═══════════════════════════════════════
  {
    const s = pres.addSlide();
    s.background = { color: C.darkBg };

    // Decorative circles
    s.addShape("oval", {
      x: 7.5, y: 3.0, w: 4.0, h: 4.0,
      fill: { color: C.blue, transparency: 93 },
    });
    s.addShape("oval", {
      x: -1.5, y: -0.5, w: 3.0, h: 3.0,
      fill: { color: C.pink, transparency: 93 },
    });

    // Thanks text
    s.addText("感谢聆听", {
      x: 1.5, y: 1.5, w: 7.0, h: 1.0,
      fontSize: 44, fontFace: "Arial", color: C.white, bold: true, margin: 0,
    });
    s.addText("THANK YOU", {
      x: 1.5, y: 2.4, w: 7.0, h: 0.5,
      fontSize: 16, fontFace: "Arial", color: C.muted,
      charSpacing: 8, margin: 0,
    });

    s.addShape("rect", {
      x: 1.5, y: 3.15, w: 2.0, h: 0.04,
      fill: { color: C.pink },
    });

    s.addText("请各位老师批评指正", {
      x: 1.5, y: 3.4, w: 7.0, h: 0.4,
      fontSize: 16, fontFace: "Arial", color: C.lightText, margin: 0,
    });

    // Bottom info
    s.addText([
      { text: "第7组  |  张柒鑫 · 李欣灿 · 湛峰 · 李方平 · 唐俊杰 · 刘俊东 · 夏天爱", options: { breakLine: true } },
      { text: "2023级计科1班  |  指导教师：刘友缘", options: { breakLine: true } },
      { text: "github.com/inkpear/Bilibili-Hot-Video-Analysis-Platform", options: {} },
    ], {
      x: 0.8, y: 4.4, w: 8.4, h: 0.9,
      fontSize: 11, fontFace: "Arial", color: C.muted,
      align: "center", lineSpacingMultiple: 1.6, margin: 0,
    });
  }

  // ─── Write file ───
  const outPath = "/Volumes/code/GIT/Bilibili-Hot-Video-Analysis-Platform/docs/B站热门视频分析平台-答辩.pptx";
  await pres.writeFile({ fileName: outPath });
  console.log("PPTX written to: " + outPath);
}

build().catch(e => { console.error(e); process.exit(1); });
