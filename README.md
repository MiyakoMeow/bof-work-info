# BOF Table Fetch 项目

一个用于抓取BMS（Be-Music Source）活动表格数据的Rust项目，支持从多个URL获取数据并输出为TOML格式。

> **特别说明**: 该项目专门为DEE2会场设计，适用于DEE2会场的BMS活动数据抓取需求。

## 项目结构

```
bof-table-fetch/
├── Cargo.toml                 # 工作空间配置文件
├── events.toml               # 事件配置文件（包含所有BMS活动事件）
├── events/                   # 各事件的数据文件目录
│   ├── BOF2005.toml         # BOF2005活动数据
│   ├── BOF2006.toml         # BOF2006活动数据
│   ├── BOF2008.toml         # BOF2008活动数据
│   ├── BOF2009.toml         # BOF2009活动数据
│   ├── BOF2010.toml         # BOF2010活动数据
│   ├── BOF2011.toml         # BOF2011活动数据
│   ├── BOF2012.toml         # BOF2012活动数据
│   ├── BOF2013.toml         # BOF2013活动数据
│   ├── BOFET.toml           # BOFET活动数据
│   ├── BOFNT.toml           # BOFNT活动数据
│   ├── BOFTT.toml           # BOFTT活动数据
│   ├── BOFU2015.toml        # BOFU2015活动数据
│   ├── BOFU2016.toml        # BOFU2016活动数据
│   ├── BOFU2017.toml        # BOFU2017活动数据
│   ├── BOFXV.toml           # BOFXV活动数据
│   ├── BOFXVI.toml          # BOFXVI活动数据
│   ├── BOFXVII.toml         # BOFXVII活动数据
│   ├── G2R2014.toml         # G2R2014活动数据
│   └── G2R2018.toml         # G2R2018活动数据
├── fetcher/                  # 核心抓取工具
│   ├── Cargo.toml           # 项目依赖配置
│   ├── README.md            # 工具使用说明
│   └── src/
│       └── main.rs          # 主程序源码
├── downloader/               # 作品下载工具
│   ├── Cargo.toml           # 项目依赖配置
│   ├── README.md            # 工具使用说明
│   └── src/
│       └── main.rs          # 主程序源码
├── .github/                  # GitHub Actions配置
│   └── workflows/
│       ├── update-events.yml # 自动更新事件数据的工作流
│       └── cleanup-merged-pr-branches.yml # 清理合并分支的工作流
└── target/                   # 编译输出目录
    ├── debug/               # 调试版本
    └── release/             # 发布版本
```

## 项目组件

### 1. 工作空间配置 (`Cargo.toml`)
- 定义Rust工作空间，包含`fetcher`子项目
- 使用Rust 2024版本
- 配置默认成员为`fetcher`

### 2. 事件配置 (`events.toml`)
- 包含所有BMS活动事件的配置信息
- 每个事件包含`key`（事件名称）和`event_id`（事件ID）
- 支持从BOF2005到BOFXVII等多个历史活动

### 3. 数据文件目录 (`events/`)
- 存储各个活动的具体数据文件
- 每个文件包含该活动的所有参赛作品信息
- 数据格式为TOML，包含作品序号、作者、标题、大小、下载地址等信息

### 4. 核心抓取工具 (`fetcher/`)
- 基于Rust开发的命令行工具
- 支持从多个URL批量抓取BMS表格数据
- 具备智能列映射、编码检测、去重等功能
- 详细使用说明请参考 `fetcher/README.md`

### 5. 作品下载工具 (`downloader/`)
- 基于Rust开发的命令行下载工具
- 支持从events/*.toml文件读取作品信息并下载
- 支持多种下载链接类型（直链、Google Drive、Dropbox、OneDrive、MediaFire等）
- 支持交互模式选择下载链接
- 详细使用说明请参考 `downloader/README.md`

### 6. 自动化工作流 (`.github/workflows/`)
- **update-events.yml**: 每6小时自动运行，更新所有事件数据
- **cleanup-merged-pr-branches.yml**: 自动清理已合并的PR分支

## 技术特性

### 数据抓取
- 🔍 智能表格结构检测和解析
- 🌐 支持多URL批量处理
- 🔄 自动去重和编码检测
- 📝 支持多种输入方式（配置文件、stdin）

### 数据处理
- 📤 支持多种输出方式（stdout、文件）
- 🐛 完整的日志系统
- 📊 TOML格式输出，便于后续处理

### 作品下载
- 🔗 支持多种下载链接类型（直链、Google Drive、Dropbox、OneDrive、MediaFire等）
- 🆔 支持分享ID格式和完整URL格式
- 🔍 自动从完整分享链接中提取分享ID（支持多种Google Drive和Dropbox格式）
- 🎯 支持按作品编号筛选下载
- 🤝 交互模式支持多链接选择
- 📁 自动创建输出目录和清理文件名

### 自动化
- ⏰ 定时自动更新数据
- 🔄 GitHub Actions自动化工作流
- 📈 支持增量更新和全量更新

## 开发环境

### 依赖要求
- Rust 1.70+ (支持2024版本)
- Cargo包管理器

### 主要依赖
- `scraper`: HTML解析
- `toml`: TOML格式支持
- `surf`: HTTP客户端
- `clap`: 命令行参数解析
- `log` + `env_logger`: 日志系统
- `anyhow`: 错误处理
- `encoding_rs`: 字符编码检测
- `regex`: 正则表达式支持

## 快速开始

1. **克隆项目**
   ```bash
   git clone <repository-url>
   cd bof-table-fetch
   ```

2. **构建项目**
   ```bash
   cargo build --release
   ```

3. **运行抓取工具**
   ```bash
   # 抓取所有事件数据
   cargo run -p fetcher -- --output all_events.toml
   
   # 查看详细使用说明
   cargo run -p fetcher -- --help
   ```

4. **下载作品文件**
   ```bash
   # 下载BOFTT活动的所有作品
   cargo run -p downloader -- --event events/BOFTT.toml
   
   # 下载特定作品
   cargo run -p downloader -- --event events/BOFTT.toml --entries "1,3,5"
   
   # 查看下载工具使用说明
   cargo run -p downloader -- --help
   ```

5. **查看特定事件数据**
   ```bash
   # 查看BOF2005数据
   cat events/BOF2005.toml
   ```

## 项目维护

- 数据通过GitHub Actions自动更新
- 支持手动触发数据更新
- 所有数据变更通过Pull Request进行管理
- 定期清理已合并的分支

## 贡献指南

1. Fork项目
2. 创建功能分支
3. 提交更改
4. 创建Pull Request

详细的使用说明请参考 `fetcher/README.md`。
