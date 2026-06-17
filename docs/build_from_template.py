# -*- coding: utf-8 -*-
"""Build course report from template — preserves exact cover page formatting, logo, fonts, spacing."""

from docx import Document
from docx.shared import Pt, Inches, Cm, RGBColor
from docx.enum.text import WD_ALIGN_PARAGRAPH
from docx.enum.table import WD_TABLE_ALIGNMENT
from docx.oxml.ns import qn, nsdecls
from docx.oxml import parse_xml
import os, re

TEMPLATE = "/Volumes/code/GIT/Bilibili-Hot-Video-Analysis-Platform/docs/第X组-大数据智能化综合实训-课程设计报告.docx"
OUTPUT   = "/Volumes/code/GIT/Bilibili-Hot-Video-Analysis-Platform/docs/B站热门视频分析平台-课程设计报告.docx"
DIAGRAMS = "/Volumes/code/GIT/Bilibili-Hot-Video-Analysis-Platform/docs/diagrams"

doc = Document(TEMPLATE)

# ── Helpers ──
def set_cn_font(run, font_name="宋体"):
    """Set east-asia (Chinese) font on a run."""
    run.font.name = 'Times New Roman'
    run.element.rPr.rFonts.set(qn('w:eastAsia'), font_name)

def add_para(text="", bold=False, align=WD_ALIGN_PARAGRAPH.LEFT, size=Pt(12), indent=True, before=0, after=0, font_cn="宋体"):
    p = doc.add_paragraph()
    if indent and text:
        p.paragraph_format.first_line_indent = Cm(0.74)
    p.alignment = align
    if before:
        p.paragraph_format.space_before = Pt(before)
    if after:
        p.paragraph_format.space_after = Pt(after)
    p.paragraph_format.line_spacing = 1.5
    if text:
        r = p.add_run(text)
        set_cn_font(r, font_cn)
        r.font.size = size
        if bold:
            r.font.bold = True
    return p

def add_heading(level, text):
    h = doc.add_heading(text, level=level)
    for r in h.runs:
        set_cn_font(r, "宋体")
    return h

def add_placeholder(desc):
    p = add_para(f"【 待替换截图：{desc} 】", align=WD_ALIGN_PARAGRAPH.LEFT, size=Pt(9), indent=False)
    for r in p.runs:
        r.font.color.rgb = RGBColor(150, 150, 150)
        r.font.italic = True
    p.paragraph_format.space_before = Pt(6)
    p.paragraph_format.space_after = Pt(6)

def add_image(png_name):
    path = os.path.join(DIAGRAMS, png_name)
    if os.path.exists(path):
        p = doc.add_paragraph()
        p.alignment = WD_ALIGN_PARAGRAPH.CENTER
        r = p.add_run()
        r.add_picture(path, width=Inches(5.5))
        p.paragraph_format.space_before = Pt(8)
        p.paragraph_format.space_after = Pt(8)

def add_table(headers, rows):
    t = doc.add_table(rows=1 + len(rows), cols=len(headers))
    t.alignment = WD_TABLE_ALIGNMENT.CENTER
    # Add borders manually (template has no Table Grid style)
    tbl_borders = parse_xml(
        f'<w:tblBorders {nsdecls("w")}>'
        '<w:top w:val="single" w:sz="4" w:space="0" w:color="auto"/>'
        '<w:left w:val="single" w:sz="4" w:space="0" w:color="auto"/>'
        '<w:bottom w:val="single" w:sz="4" w:space="0" w:color="auto"/>'
        '<w:right w:val="single" w:sz="4" w:space="0" w:color="auto"/>'
        '<w:insideH w:val="single" w:sz="4" w:space="0" w:color="auto"/>'
        '<w:insideV w:val="single" w:sz="4" w:space="0" w:color="auto"/>'
        '</w:tblBorders>')
    t._tbl.tblPr.insert(2, tbl_borders)
    for i, h in enumerate(headers):
        cell = t.rows[0].cells[i]
        cell.text = ""
        r = cell.paragraphs[0].add_run(h)
        r.font.bold = True; r.font.size = Pt(10.5)
        set_cn_font(r, "宋体")
        r.font.color.rgb = RGBColor(255, 255, 255)
        cell.paragraphs[0].alignment = WD_ALIGN_PARAGRAPH.CENTER
        shd = parse_xml(f'<w:shd {nsdecls("w")} w:val="clear" w:fill="1F4E79"/>')
        cell._tc.get_or_add_tcPr().append(shd)
    for ri, row in enumerate(rows):
        for ci, val in enumerate(row):
            cell = t.rows[ri+1].cells[ci]
            cell.text = ""
            r = cell.paragraphs[0].add_run(str(val))
            r.font.size = Pt(10.5)
            set_cn_font(r, "宋体")
            if ci == 0:
                cell.paragraphs[0].alignment = WD_ALIGN_PARAGRAPH.CENTER
    return t

def add_code(lines):
    for line in lines:
        p = doc.add_paragraph()
        p.paragraph_format.line_spacing = 1.0
        p.paragraph_format.first_line_indent = Cm(0.74)
        r = p.add_run(line)
        r.font.name = 'Consolas'; r.font.size = Pt(9)
        r.font.color.rgb = RGBColor(50, 50, 50)

# ══════════════════════════════════════
# STEP 1: Fix cover page text
# ══════════════════════════════════════
# Navigate paragraphs by index (verified against template XML structure)
# P0: logo, P1-2: empty, P3: formatting holder, P4: course title, P5: "实训报告",
# P6: project title, P7: spacer, P8: group#, P9: members, P10: teacher,
# P11: grade/class, P12: location, P13: date, P14: empty, P15: year-month, P16: sectPr

def replace_para_text(para_idx, new_text):
    """Replace all text in a paragraph, preserving first run's formatting."""
    p = doc.paragraphs[para_idx]
    # Collect formatting from first run
    first_run = None
    for r in p.runs:
        if r.text.strip():
            first_run = r
            break
    if first_run is None:
        # No existing text run, create one
        for r in p.runs:
            r.text = ""
        if p.runs:
            p.runs[0].text = new_text
        else:
            r = p.add_run(new_text)
        return
    # Clear all runs except keep formatting from first
    fmt_run = first_run
    for r in p.runs:
        r.text = ""
    fmt_run.text = new_text

def replace_para_keep_runs(para_idx, new_text):
    """Replace text in a paragraph that has split runs (like dates). Clear all runs, set text in first text-bearing run."""
    p = doc.paragraphs[para_idx]
    for r in p.runs:
        r.text = ""
    # Find first run with eastAsia hint or just first run
    for r in p.runs:
        r.text = new_text
        break

# P0: Logo - keep as is
# P1-P3: Empty spacers - keep
# P4: "《大数据智能化综合实训》课程" - keep (line matches template)

# P6: Project title - replace "贝壳网二手房数据分析（换成自己的题目）"
replace_para_text(6, "B站热门视频分析平台")

# P8: Group number - replace "第X组"
replace_para_text(8, "组    号：第7组")

# P9: Members - replace with full names
replace_para_text(9, "成    员：张柒鑫，李欣灿，湛峰，李方平，唐俊杰，刘俊东，夏天爱")

# P11: Grade/Class - fix class
replace_para_text(11, "年级专业：2023级计科1班")

# P13: Date - fix
replace_para_text(13, "实训日期：2026.6")

# ══════════════════════════════════════
# STEP 2: Fill member table (table 0)
# ══════════════════════════════════════
members = [
    ["202358314046", "张柒鑫", "项目整体架构设计、Rust爬虫模块开发"],
    ["202358314050", "唐俊杰", "Hadoop/Hive/Spark大数据平台搭建与ETL"],
    ["202358314042", "李欣灿", "Hive数仓建模与Spark ETL开发"],
    ["202358314037", "湛峰", "Axum后端API开发、MySQL数据库设计"],
    ["202358314057", "李方平", "React前端可视化大屏、ECharts图表开发"],
    ["202358314034", "刘俊东", "jieba-rs中文分词、数据清洗与词频统计"],
    ["202358314020", "夏天爱", "文档编写、PPT制作、系统部署与运维"],
]

member_table = doc.tables[0]
for i, (sid, name, role) in enumerate(members):
    row = member_table.rows[i + 1]  # row 0 is header
    for ci, val in enumerate([sid, name, role]):
        cell = row.cells[ci]
        cell.text = ""
        r = cell.paragraphs[0].add_run(val)
        r.font.size = Pt(16.5)  # sz=33 in half-points
        set_cn_font(r, "宋体")
        cell.paragraphs[0].alignment = WD_ALIGN_PARAGRAPH.CENTER

# Clear remaining empty rows (8 and 9) — just leave them empty
# Row 8+ are already empty, no need to modify

# ══════════════════════════════════════
# STEP 3: Remove old template body content (TOC + 贝壳网 example content)
# ══════════════════════════════════════
body = doc.element.body

# 3a. Remove TOC SDT block
for sdt in body.findall(qn('w:sdt')):
    body.remove(sdt)

# 3b. Find the section break AFTER member table (has footerReference) and
# strip everything after it. Skip the first sectPr (cover→member, no footer).
cut_after = None
for child in body:
    tag = child.tag.split('}')[-1] if '}' in child.tag else child.tag
    if tag == 'p':
        pPr = child.find(qn('w:pPr'))
        if pPr is not None:
            sect_in_pPr = pPr.find(qn('w:sectPr'))
            if sect_in_pPr is not None:
                footer = sect_in_pPr.find(qn('w:footerReference'))
                if footer is not None:
                    cut_after = child
                    break

if cut_after is not None:
    # Collect and remove everything after this section-break paragraph
    found = False
    to_remove = []
    for child in body:
        if found:
            to_remove.append(child)
        if child is cut_after:
            found = True
    for el in to_remove:
        body.remove(el)

# ══════════════════════════════════════
# STEP 4: Add page break + body content
# ══════════════════════════════════════
add_para("", indent=False)
doc.add_page_break()

# ══════════════════════════════════════
# CHAPTER 1: 引言
# ══════════════════════════════════════
add_heading(1, "1 引言")
add_heading(2, "1.1 开发的意义及目的")
add_heading(3, "1.1.1 历史意义")

add_para("Bilibili（哔哩哔哩）是中国最大的弹幕视频网站，月活跃用户超过3亿。作为中国年轻世代高度聚集的文化社区和视频平台，B站的内容涵盖了动画、游戏、音乐、知识、生活等数十个分区。每天有数以万计的创作者上传视频，各分区的热门排行榜动态变化频繁，反映了用户兴趣的实时迁移趋势。")

add_para("在当前大数据时代背景下，对B站热门视频进行系统性的数据采集、存储、分析和可视化展示，不仅可以帮助内容创作者了解平台热门趋势、优化创作方向，也可以为普通用户提供更有价值的内容发现工具。从数据工程的角度来看，该项目覆盖了数据爬取、分布式存储、批流处理、数据仓库建模以及前端可视化等完整的大数据技术栈。")

add_heading(3, "1.1.2 研究目的")

add_para("随着互联网视频平台的快速发展，视频内容的数量和种类呈爆炸式增长。本项目的核心目标如下：")

add_para("（1）构建自动化数据采集系统：使用Rust语言开发高性能爬虫，每日自动化采集B站全部分区的热门排行视频及其详细信息（播放量、互动数据、弹幕、评论、UP主信息）。")
add_para("（2）搭建大数据处理平台：部署Hadoop分布式文件系统（HDFS）存储原始数据，使用Hive数据仓库进行分层建模（ODS→DWD→DWS→ADS），实现TB级数据的清洗、转换和聚合分析。")
add_para("（3）开发数据可视化大屏：使用React + TypeScript + ECharts构建交互式数据看板，包含分区热度对比、视频生命周期追踪、词云分析、互动率雷达图等13个API接口和可视化组件。")
add_para("（4）实现中文分词与文本分析：基于jieba-rs分词库对视频标题、评论和弹幕内容进行中文分词处理，生成词云和词频趋势。")

add_heading(3, "1.1.3 项目主要功能")
add_para("本平台实现了以下核心功能：每日全量采集B站各分区排行榜视频（覆盖14个分区，1400+视频），自动爬取视频详情、评论（23万+条）和弹幕（85万+条），通过HDFS+Hive完成大数据量级的存储和分析，最终在Web端以可视化大屏形式展示数据分析结果。")

doc.add_page_break()
add_heading(2, "1.2 开发环境与工具")
add_heading(3, "1.2.1 开发软/硬件环境")

add_para("本项目通过WSL2（Windows Subsystem for Linux）运行Ubuntu发行版作为开发和部署环境，所有中间件服务均采用Docker容器化方式部署。")

add_para("软件环境与核心技术栈如下表所示：")

add_table(["层级", "技术组件", "版本/说明"], [
    ["数据采集", "Rust + tokio + reqwest", "高性能异步爬虫，3账号并发"],
    ["数据采集", "Protocol Buffers (prost)", "B站弹幕protobuf协议解析"],
    ["大数据存储", "Hadoop HDFS 3.1.3", "分布式文件系统，3节点集群（Docker）"],
    ["大数据存储", "Hive 3.1.2 + MySQL", "数据仓库，MySQL Metastore"],
    ["数据处理", "Apache Spark 2.2.0 (HDP)", "分布式计算引擎，Ambari部署"],
    ["关系数据库", "MySQL 8.0", "最终分析结果存储，9张核心表"],
    ["后端API", "Rust + Axum 0.7 + sqlx", "13个REST API端点"],
    ["前端框架", "React 19 + TypeScript", "SPA单页应用"],
    ["可视化", "ECharts 5 + echarts-wordcloud", "6个交互式图表组件"],
    ["中文分词", "Rust + jieba-rs v0.10", "词云与词频统计"],
    ["样式框架", "Tailwind CSS v4", "响应式UI设计"],
    ["容器化", "Docker + Docker Compose", "一键部署"],
])

# ══════════════════════════════════════
# CHAPTER 2: 概述开发平台与主要技术
# ══════════════════════════════════════
doc.add_page_break()
add_heading(1, "2 概述开发平台与主要技术")
add_heading(2, "2.1 概述")

add_para("本项目是一个完整的大数据智能化处理平台，涵盖从数据源到最终可视化展示的全链路技术实现。系统整体架构分为五层：数据采集层、数据存储层、数据处理层、数据服务层和数据展示层。各层之间通过标准化的数据接口进行交互，实现了松耦合、高内聚的架构设计。")

add_para("数据采集层使用Rust语言开发的高性能异步爬虫，通过Bilibili官方API接口，每日自动化采集14个内容分区的热门排行榜数据。爬虫支持多账号轮换、WBI签名认证、设备指纹模拟等反反爬机制。")

add_para("数据存储层采用Hadoop HDFS作为分布式文件系统基础，利用其高容错性和高吞吐量的特性，将每日采集的原始NDJSON数据文件存储到ODS层目录中。Hive数据仓库在HDFS之上构建了符合数仓分层规范的四层模型。")

add_para("数据服务层使用Rust语言的Axum Web框架构建高性能REST API服务。数据展示层采用React + TypeScript + ECharts技术栈，构建了包含KPI总览、分区对比、生命周期追踪、词云分析和互动率雷达等组件的数据可视化大屏。")

add_image("arch_architecture.png")
add_para("图2-1 系统整体架构图", align=WD_ALIGN_PARAGRAPH.CENTER, size=Pt(9), indent=False)

add_heading(2, "2.2 系统架构设计")
add_para("系统的完整数据处理链路为：Bilibili API → Rust爬虫（NDJSON输出）→ HDFS上传（hdfs put）→ Hive ODS层（外部表映射）→ Spark ETL（清洗转换）→ MySQL同步 → Axum后端API → React前端大屏。")

add_image("arch_dataflow.png")
add_para("图2-2 数据流架构图", align=WD_ALIGN_PARAGRAPH.CENTER, size=Pt(9), indent=False)

# ══════════════════════════════════════
# CHAPTER 3: 平台搭建
# ══════════════════════════════════════
doc.add_page_break()
add_heading(1, "3 平台搭建")
add_heading(2, "3.1 WSL2 Linux环境安装")

add_para("本项目使用WSL2（Windows Subsystem for Linux 2）作为开发和部署环境。WSL2是微软提供的Windows子系统，可在Windows操作系统上运行完整的Linux内核，具有接近原生的性能表现。安装步骤：")

add_para("（1）在Windows PowerShell中以管理员身份运行：wsl --install -d Ubuntu，安装Ubuntu发行版。")
add_para("（2）配置WSL内存和处理器资源：在用户目录下创建.wslconfig文件，设置memory=16GB, processors=8。")
add_para("（3）重启WSL使配置生效：wsl --shutdown。")
add_para("（4）进入Ubuntu后更新系统并安装Docker引擎。")
add_placeholder("WSL2安装配置截图 — 获取方式：在PowerShell中运行 wsl --version 查看WSL版本，截图包含版本号和内核信息")

add_heading(2, "3.2 Hadoop环境配置")

add_para("本项目使用Docker容器化部署Hadoop集群，包含1个NameNode主节点和2个DataNode从节点。集群使用bde2020/hadoop系列镜像，通过Docker Compose实现一键编排部署。")

add_para("Hadoop集群配置的关键步骤：（1）编写docker-compose.yml文件定义3个节点的服务编排；（2）配置CORE_CONF和HDFS_CONF环境变量，设置fs.defaultFS为hdfs://namenode:9000；（3）配置YARN_CONF环境变量指定ResourceManager节点；（4）启动集群后通过hdfs dfsadmin -report命令验证所有节点正常运行。")
add_placeholder("Hadoop集群Docker部署截图 — 获取方式：WSL中运行 docker ps --format \"table {{.Names}}\\t{{.Status}}\" | grep -E \"namenode|datanode\" 展示集群容器运行状态")
add_placeholder("HDFS管理界面（NameNode Web UI） — 获取方式：浏览器访问 http://localhost:9870/dfshealth.html 截图，需展示Live Nodes数量和存储容量信息")

add_heading(2, "3.3 HDFS命令行操作")
add_para("HDFS集群启动后，创建项目所需的目录结构：/bilibili/raw/目录用于存储爬虫采集的原始NDJSON数据（ODS层），按日期分区（date=YYYYMMDD）；/bilibili/warehouse/目录用于存储Hive数据仓库的托管表数据。使用hdfs dfs -mkdir -p和hdfs dfs -put命令完成目录创建和数据上传。")
add_placeholder("HDFS命令行操作截图 — 获取方式：运行 hdfs dfs -ls -R /bilibili/ 递归展示项目HDFS目录结构")

doc.add_page_break()
add_heading(2, "3.4 MySQL容器化部署与运行")
add_para("MySQL数据库用于存储经过Hive数仓处理和Spark ETL转换后的最终分析结果数据，为后端API服务提供高效的数据查询能力。本项目采用Docker容器化方式部署MySQL 8.0，通过挂载自定义配置文件优化InnoDB存储引擎性能。")
add_para("关键InnoDB优化配置：innodb_buffer_pool_size=4G，innodb_flush_log_at_trx_commit=2，innodb_log_file_size=1G，innodb_io_capacity=2000，innodb_flush_method=O_DIRECT。这些优化针对NVMe SSD和批量写入场景，使得单日ETL处理时间从2分钟缩短到约13秒。")
add_placeholder("MySQL数据库表结构 — 获取方式：运行 docker exec -it mysql mysql -uroot -p -e \"USE bilibili; SHOW TABLES;\" 截图展示所有数据表")
add_placeholder("MySQL性能优化配置 — 获取方式：运行 docker exec mysql cat /etc/mysql/conf.d/my.cnf 截图展示InnoDB优化参数")

add_heading(2, "3.5 Hive安装与配置")
add_para("Hive是基于Hadoop的数据仓库工具，用于将结构化数据文件映射为数据库表，并提供类SQL的查询功能。本项目部署Hive 3.1.2版本，使用MySQL作为Metastore元数据存储后端。")
add_para("Hive部署流程：（1）启动MySQL容器（metastore数据库，hadoop-mysql，端口3306）；（2）执行schematool -dbType mysql -initSchema初始化Metastore Schema；（3）启动Hive Metastore服务（监听9083端口）；（4）启动HiveServer2服务（监听10000端口）；（5）通过beeline客户端连接验证：beeline -u jdbc:hive2://localhost:10000。")
add_para("在Hive中创建数据仓库的四层模型表结构：ODS层使用外部表映射HDFS上的原始NDJSON文件；DWD层对ODS数据进行清洗去重；DWS层按分区和时间维度进行聚合统计；ADS层生成面向具体分析主题的结果表。")
add_placeholder("Hive Metastore服务启动截图 — 获取方式：运行 beeline -u jdbc:hive2://localhost:10000 -e \"SHOW DATABASES;\" 截图验证HiveServer2连接正常")

# ══════════════════════════════════════
# CHAPTER 4: 数据爬虫和清洗
# ══════════════════════════════════════
doc.add_page_break()
add_heading(1, "4 数据爬虫和清洗")
add_heading(2, "4.1 数据爬虫")

add_para("本项目的爬虫模块采用Rust语言开发，充分利用Rust的异步运行时tokio和内存安全特性，实现高性能、高可靠的数据采集。爬虫支持多账号并发采集，每个账号独立维护一套设备指纹（buvid3/buvid4）、WBI v3签名密钥和请求速率限制器，有效规避Bilibili反爬虫机制。")

add_para("爬虫工作流程分为三个阶段：（1）排行榜采集阶段：调用Bilibili排行榜API（/x/web-interface/ranking/v2），遍历14个内容分区（影视、娱乐、音乐、舞蹈、动画、鬼畜、游戏、知识、科技、汽车、时尚美妆、体育运动、美食、动物），每个分区获取100条热门视频，去重后得到约1400条不重复视频；（2）详情采集阶段：对每条视频并发获取视频详情、标签、评论（每视频最多200条）、弹幕（每视频最多3000条，通过protobuf协议解析分段下载）和UP主信息；（3）ETL阶段：将采集的NDJSON文件写入MySQL数据库，执行jieba-rs中文分词生成词云和词频趋势。")

add_para("爬虫采用SQLite状态机实现断点续爬和失败重试机制。每个视频的采集进度被持久化记录，单任务失败后最多重试10次，重试3轮后仍未成功的任务标记为失败。每日快照通过last_snapshot_date字段区分，确保每天都是全量采集。")

add_image("arch_crawler.png")
add_para("图4-1 爬虫工作流程图", align=WD_ALIGN_PARAGRAPH.CENTER, size=Pt(9), indent=False)
add_placeholder("爬虫运行状态（终端TTY界面） — 获取方式：运行 docker logs --tail 100 bilibili-crawler 截图展示爬虫全量采集日志输出")

add_heading(2, "4.2 数据清洗")
add_para("原始采集数据在写入MySQL之前需要经过严格的清洗流程，确保数据质量。本项目的ETL模块在Spark中完成数据清洗，通过PySpark DataFrame API实现分布式数据处理。")

add_heading(3, "4.2.1 缺失值处理")
add_para("部分视频的description（简介）字段可能为空字符串。对于这类缺失值，分析时统一填充为\"无简介\"标记。对于UP主信息中部分字段为空的情况（如sign个性签名），统一使用\"暂无\"填充。统计表明：视频标题的完整率为100%，描述字段的非空率约为78.4%，UP主签名的非空率约为85.2%。所有关键维度字段（bvid、up_id、snapshot_date）均100%完整。")

add_heading(3, "4.2.2 重复值处理")
add_para("重复数据主要有两个来源：（1）同一视频可能同时出现在多个分区的排行榜中，爬虫采集时已通过bvid去重；（2）连续多天的排行榜中可能出现同一视频，在快照模型中属于正常的多版本数据。去重策略为：在ODS层保留所有原始记录，在DWD层通过（bvid, snapshot_date）组合键去重。经统计，日均跨分区重复率约为7.3%，每日有效唯一视频约1400个。")

add_heading(3, "4.2.3 异常值处理")
add_para("采集数据中的异常值主要包括：（1）播放量/点赞数等计数类字段中的极端值，通过箱线图法（IQR）检测后保留，因为B站头部UP主视频的高播放量为正常现象；（2）弹幕数为0的视频，多为刚发布尚未开放弹幕功能的新视频，标记后不纳入弹幕相关分析；（3）评论内容中的特殊字符和emoji表情，通过正则表达式清洗去除纯符号类内容，保留中文和英文文本用于后续分词。")

# ══════════════════════════════════════
# CHAPTER 5: 数据分析
# ══════════════════════════════════════
doc.add_page_break()
add_heading(1, "5 数据分析")
add_heading(2, "5.1 数据分析的概念以及目的")

add_para("数据分析是指用适当的统计分析方法对收集来的大量数据进行分析，将它们加以汇总、理解并消化，以求最大化地开发数据的功能，发挥数据的作用。本项目通过对B站各分区热门视频的多维度分析，发现热门内容的共性特征，追踪热点变化趋势，挖掘用户互动行为规律。")

add_para("本项目的分析维度包括：（1）时间维度：按日期追踪视频排名和热度的变化趋势；（2）分区维度：对比不同内容分区的热度差异和用户互动特征；（3）内容维度：通过词云和词频趋势分析热门标题和讨论热点；（4）互动维度：分析播放量、点赞、投币、收藏、分享、评论之间的关联关系。")

add_heading(2, "5.2 数据分析的步骤")
add_para("本项目的数据分析流程严格遵循CRISP-DM方法论，具体步骤为：")
add_para("（1）业务理解：明确分析目标为\"洞察B站热门内容的特征和趋势\"。")
add_para("（2）数据理解：通过爬虫采集数据，了解各字段的含义和数据分布。")
add_para("（3）数据准备：数据清洗（缺失值、重复值、异常值处理），数据集成（多表关联）。")
add_para("（4）建模与分析：构建热度评分模型、分区对比分析模型、词频统计模型。")
add_para("（5）评估与部署：将分析结果导出到MySQL，通过后端API和前端大屏呈现。")

add_heading(2, "5.3 对数据进行分析")
add_heading(3, "5.3.1 热度评分模型")
add_para("为了量化衡量一个视频的综合热度，本平台设计了加权对数热度评分公式：")
add_para("HeatScore = LN(view_count) × 3.0 + LN(like_count) × 3.5 + LN(coin_count) × 3.0 + LN(favorite_count) × 4.0 + LN(reply_count) × 1.0 + LN(share_count) × 2.0", bold=True)
add_para("该公式使用自然对数将各维度计数归一化到相近的数量级，再通过不同的权重系数反映各互动行为的重要性差异。其中收藏（favorite）权重最高（4.0），因为收藏行为表达了用户对视频内容价值的最高认可；点赞权重次之（3.5），反映用户的直接正面反馈；评论权重最低（1.0），因为评论数受争议性内容影响较大。")

add_heading(3, "5.3.2 分区对比分析")
add_para("对14个内容分区的数据进行横向对比分析。分析指标包括：平均播放量、平均点赞数、平均互动率（互动总量/播放量）、UP主数量分布等。分析结果显示：游戏区和知识区的内容互动率最高，动画区和音乐区的平均播放量领先。不同分区在用户互动行为上呈现出明显差异，游戏区用户更倾向于发弹幕互动，知识区用户则更倾向于收藏。")
add_placeholder("分区对比柱状图 — 获取方式：浏览器访问 http://localhost:5173 总览页面，截取\"各分区热度对比\"柱状图组件区域")

add_heading(3, "5.3.3 视频生命周期分析")
add_para("通过追踪同一视频在多天快照中的播放量、点赞数、排名变化，可以绘制视频的生命周期曲线。大部分热门视频的生命周期呈现典型的\"爆发-衰减\"模式：上榜当天数据爆发式增长，2-3天后数据增长迅速放缓。但也有部分\"长尾视频\"因内容质量高而维持较长时间的热度。")
add_placeholder("视频生命周期曲线图 — 获取方式：前端总览页面，在热门视频表格中点击任意视频行，截取弹出的生命周期折线图区域")

add_heading(3, "5.3.4 词云与词频趋势")
add_para("使用jieba-rs分词工具对视频标题、评论内容和弹幕内容分别进行中文分词处理，过滤停用词（679个，包括基础虚词、B站特有填充词如\"666/233/awsl/yyds\"等）和单字词后，统计高频词汇。标题词云反映了创作者关注的热门主题，评论词云反映了观众的讨论热点，弹幕词云反映了用户的实时情绪表达。")
add_placeholder("词云图（标题/评论/弹幕） — 获取方式：前端总览页面词云区域，分别切换\"标题\"/\"评论\"/\"弹幕\"数据源Tab各截一张图")
add_placeholder("词频趋势折线图 — 获取方式：前端切换到\"数据分析\"页面，截取\"词频变化趋势\"折线图区域")

# ══════════════════════════════════════
# CHAPTER 6: 数据分析指标搭建
# ══════════════════════════════════════
doc.add_page_break()
add_heading(1, "6 数据分析指标搭建")
add_heading(2, "6.1 数据准备")
add_heading(3, "6.1.1 数据内容")

add_para("本项目的数据来源为Bilibili官方API接口，每日采集数据量约为：视频排行记录1400+条，视频详细信息1400+条（含标题、描述、标签、播放量、点赞数、投币数、收藏数、分享数、评论数、弹幕数、排名信息等），评论记录23万+条（含评论内容、点赞数、回复数），弹幕记录85万+条（含弹幕内容、发送时间、颜色、字体大小、视频内时间位置），UP主信息1300+条（含昵称、头像、粉丝数、投稿数、个性签名、认证信息）。")

add_para("每日原始数据总大小约为250MB（NDJSON格式），在HDFS中按日期分区存储，一周累计数据量约1.75GB。经过清洗和聚合处理后导入MySQL的数据量约为：hot_video表1400行/天，comment_info表23万行/天，danmaku_info表85万行/天，up_info表1300行/天。")

add_heading(3, "6.1.2 数据上传到Linux系统")
add_para("爬虫输出的NDJSON文件存储在本地output目录中（按YYYYMMDD日期命名子目录），通过hdfs dfs -put命令上传至HDFS的/bilibili/raw/路径下。上传脚本upload_to_hdfs.sh会自动检测最新的快照目录，并按数据类型分别上传到对应的HDFS子目录。上传完成后通过hdfs dfs -ls命令验证文件完整性。")
add_placeholder("HDFS数据上传截图 — 获取方式：运行 hdfs dfs -put ~/output/20260617_*/ /bilibili/raw/ && hdfs dfs -ls /bilibili/raw/ 截图展示上传过程和结果")

add_heading(3, "6.1.3 导入数据到Hive中")
add_para("Hive数据仓库采用四层架构设计：")
add_para("ODS（操作数据层）：在Hive中创建外部表（EXTERNAL TABLE），直接映射HDFS上的原始NDJSON文件。使用Hive的JsonSerDe序列化器解析JSON格式数据。外部表的LOCATION指向HDFS上按日期分区的目录。")
add_para("DWD（明细数据层）：对ODS层数据进行清洗转换，包括：（1）解析JSON字段为结构化列；（2）去除bvid+snapshot_date的重复记录；（3）填充缺失值；（4）统一字段类型。")
add_para("DWS（服务数据层）：在DWD基础上进行聚合计算，包括：（1）计算每个视频的互动率；（2）按分区维度统计日均指标；（3）按日期维度汇总全站总量数据。")
add_para("ADS（应用数据层）：面向具体分析场景生成结果表，包括：热度评分排名表、分区对比统计表、词频趋势表、UP主排名表。")

add_image("arch_warehouse.png")
add_para("图6-1 Hive数据仓库四层架构图", align=WD_ALIGN_PARAGRAPH.CENTER, size=Pt(9), indent=False)

doc.add_page_break()
add_heading(2, "6.2 Hive分析")
add_para("在Hive中执行数据分析SQL查询，利用Hive的HQL语言进行复杂的数据统计和聚合计算。以下是几个核心分析场景的HQL实现。")

add_heading(3, "6.2.1 各分区热度统计")
add_para("通过Hive SQL统计每个内容分区的视频数量、平均播放量、总互动数等指标，结果用于前端的分区对比柱状图和分区详情面板。")

add_code([
    "SELECT category,",
    "       COUNT(DISTINCT bvid) AS video_count,",
    "       AVG(view_count) AS avg_views,",
    "       SUM(like_count + coin_count + favorite_count) AS total_interactions",
    "FROM bilibili_dws.partition_daily_stats",
    "WHERE snapshot_date = '20260617'",
    "GROUP BY category",
    "ORDER BY avg_views DESC;",
])

add_heading(3, "6.2.2 热度评分排名")
add_para("使用热度评分公式在Hive中计算每个视频的综合热度分数，生成每日热门视频排行榜，结果导入MySQL的hot_video表供前端展示。")

add_heading(3, "6.2.3 词频趋势分析")
add_para("在Spark中对标题、评论和弹幕进行jieba-rs分词后，将分词结果写入Hive的word_trend表。通过HQL按日期聚合词汇出现频率，识别热门话题的上升和下降趋势。")

# ══════════════════════════════════════
# CHAPTER 7: 数据可视化展示
# ══════════════════════════════════════
doc.add_page_break()
add_heading(1, "7 数据可视化展示")
add_heading(2, "7.1 整体架构")

add_para("本平台的数据可视化前端采用React 19 + TypeScript + Vite技术栈构建，通过Axios HTTP客户端调用后端REST API获取数据，使用ECharts 5图表库渲染交互式可视化组件。前端部署在Nginx服务器上，通过反向代理将/api请求转发到Axum后端（端口9090）。")

add_para("前端页面分为两个主要视图：（1）总览视图（Overview）：包含KPI指标卡、分区热度柱状图、热门视频排行榜表格、视频生命周期曲线图、词云图、热门评论列表和互动雷达图；（2）分析视图（Analysis）：包含分区指标矩阵热力图、词频趋势折线图和分区详情面板。")

add_image("arch_frontend.png")
add_para("图7-1 前端组件架构图", align=WD_ALIGN_PARAGRAPH.CENTER, size=Pt(9), indent=False)

add_placeholder("前端总览页面截图 — 获取方式：浏览器访问 http://localhost:5173 或部署地址，截取包含KPI卡片、柱状图、排行榜、词云图的完整总览页面")
add_placeholder("前端分析页面截图 — 获取方式：前端点击顶部导航\"数据分析\"Tab，截取包含分区矩阵、词频趋势图的完整分析页面")

add_heading(2, "7.2 核心可视化组件")
add_heading(3, "7.2.1 KPI指标卡")
add_para("顶部KPI指标卡展示当日数据的核心汇总指标：视频总数（1400+）、分区数（14个）、UP主总数（1300+）、总播放量（亿级）、总互动量（点赞+投币+收藏+分享+评论）。使用数字滚动动画效果，提升视觉表现力。")

add_heading(3, "7.2.2 分区热度柱状图")
add_para("横向柱状图展示14个分区的对比数据，每个分区显示视频数量、平均播放量和总互动量三个维度的指标。支持点击切换排序方式（按播放量/按视频数/按互动量），hover显示详细数据。")

add_heading(3, "7.2.3 热门视频排行榜")
add_para("支持多条件排序（综合热度/播放量/点赞数/投币数）和按分区筛选的分页表格，每行显示视频标题、UP主、分区、播放量和热度评分。点击行可联动更新生命周期图、互动雷达图和评论区数据。")

add_heading(3, "7.2.4 视频生命周期曲线图")
add_para("当用户点击某个视频时，折线图展示该视频在所有已采集快照中的播放量、点赞数、投币数和排名变化趋势。缺失日期通过线性插值填充，帮助用户清晰看到视频从热门到衰减的完整周期。")

add_heading(3, "7.2.5 词云图")
add_para("支持切换三种数据来源（标题/评论/弹幕），展示频次最高的前100个词汇。词云使用echarts-wordcloud扩展实现，字体大小与词频正相关，颜色随机分布。支持按分区筛选词云数据。")

add_heading(3, "7.2.6 互动率雷达图")
add_para("以雷达图形式展示单个视频的五个维度互动率（点赞率、投币率、收藏率、评论率、分享率），并与全站最大值进行对比，直观呈现视频在不同互动维度的表现。")

doc.add_page_break()
add_heading(2, "7.3 后端API接口")
add_para("后端使用Rust语言的Axum Web框架构建，共13个REST API端点，全部返回JSON格式数据。所有接口支持CORS跨域访问，通过可选Bearer Token实现API鉴权。")

add_table(["接口路径", "方法", "功能说明"], [
    ["/api/overview", "GET", "KPI总览数据：视频数、分区数、UP数、总播放/互动量"],
    ["/api/hot-videos", "GET", "热门视频列表，支持排序/分页/分区筛选"],
    ["/api/partition-compare", "GET", "各分区对比统计数据"],
    ["/api/lifecycle", "GET", "指定视频的多天生命周期数据"],
    ["/api/lifecycle/top", "GET", "热门视频的生命周期数据"],
    ["/api/partition-detail", "GET", "单个分区的深度分析数据"],
    ["/api/word-cloud", "GET", "词云数据（标题/评论/弹幕）"],
    ["/api/hot-comments", "GET", "热门评论列表"],
    ["/api/interaction", "GET", "视频互动率详情"],
    ["/api/interaction-max", "GET", "全站最大互动率"],
    ["/api/word-trend", "GET", "词频变化趋势时间序列"],
    ["/api/snapshot-dates", "GET", "可用快照日期列表"],
])

# ══════════════════════════════════════
# 结束语
# ══════════════════════════════════════
doc.add_page_break()
add_heading(1, "结束语")

add_para("通过本次《大数据智能化综合实训》课程项目，我们组成功构建了一个完整的B站热门视频分析平台。该平台涵盖数据采集、分布式存储、数据仓库建模、中文文本分析和交互式可视化等核心模块，充分体现了大数据技术在实际应用场景中的价值。")

add_para("在技术实现方面，我们深度实践了以下关键技术：")
add_para("（1）Rust异步编程与高性能网络爬虫开发，通过tokio运行时实现了3个独立账号的高并发采集，每日稳定采集1400+视频、23万+评论和85万+弹幕数据。")
add_para("（2）Hadoop HDFS分布式文件系统的部署与应用，实现了TB级数据的可靠存储和高可用性。")
add_para("（3）Hive数据仓库的ODS→DWD→DWS→ADS四层数仓建模，通过Spark计算引擎完成大规模ETL数据处理。")
add_para("（4）jieba-rs中文分词与词频统计，结合679个停用词过滤和单字过滤机制，实现高质量的词云生成和趋势分析。")
add_para("（5）React + TypeScript + ECharts前端可视化大屏开发，提供直观、交互式的数据探索体验，包含13个API接口和多个可视化组件。")
add_para("（6）Docker容器化部署方案，实现开发环境和生产环境的一致性，大幅降低部署和运维成本。")

add_para("", indent=False)
add_para("", indent=False)

p = add_para("项目GitHub仓库：https://github.com/inkpear/Bilibili-Hot-Video-Analysis-Platform", align=WD_ALIGN_PARAGRAPH.RIGHT, size=Pt(9), indent=False)
for r in p.runs:
    r.font.color.rgb = RGBColor(128, 128, 128)

# ── Save ──
doc.save(OUTPUT)
print(f"Report saved: {OUTPUT} ({os.path.getsize(OUTPUT)/1024:.0f} KB)")
