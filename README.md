# BOF Table Fetch

一个用于抓取BMS表格数据的Rust工具，支持从多个URL获取数据并输出为TOML格式。

## 功能特性

- 🔍 自动检测和解析BMS表格结构
- 📝 支持多种输入方式：TOML配置文件、stdin、默认URL
- 📤 支持多种输出方式：stdout、指定文件
- 🐛 完整的日志系统，支持不同日志级别
- 🌐 支持多URL批量处理
- 🔄 自动去重和编码检测

## 安装

```bash
cargo build --release
```

## 使用方法

### 基本用法

```bash
# 使用默认URL，输出到stdout
cargo run

# 使用默认URL，输出到文件
cargo run -- --output data.toml

# 设置日志级别
cargo run -- --log-level debug
```

### 从TOML配置文件读取URL

创建URL配置文件 `urls.toml`：

```toml
urls = [
    "https://manbow.nothing.sh/event/event.cgi?action=URLList&event=14&end=999",
    "https://example.com/another-bms-table",
]
```

然后运行：

```bash
cargo run -- --input urls.toml --output output.toml
```

### 从stdin读取URL

```bash
echo "https://manbow.nothing.sh/event/event.cgi?action=URLList&event=14&end=999" | cargo run -- --stdin
```

或者：

```bash
cargo run -- --stdin < urls.txt
```

## 命令行参数

- `-i, --input <PATH>`: 输入TOML配置文件路径，包含要抓取的URL列表
- `-o, --output <PATH>`: 输出文件路径，如果不指定则输出到stdout
- `--stdin`: 从stdin读取URL列表（每行一个URL）
- `--log-level <LEVEL>`: 日志级别 (trace, debug, info, warn, error)，默认为info

## 输出格式

程序输出TOML格式的数据，包含以下字段：

```toml
[[entries]]
no = "1"                    # 序号
name = "cyclia"             # 作者名
title = "Cynthia"           # 曲目名
size = "3114 KB"            # 文件大小
team = "Team Name"          # 团队名（可选）
addr = [                    # 地址列表
    "http://example.com/",
    "http://example.com/download.zip",
]
```

## 日志级别

- `trace`: 最详细的日志，包括所有内部操作
- `debug`: 调试信息，包括解析过程
- `info`: 一般信息，包括处理进度
- `warn`: 警告信息
- `error`: 错误信息

## 示例

### 批量处理多个URL

```bash
# 创建配置文件
cat > urls.toml << EOF
urls = [
    "https://manbow.nothing.sh/event/event.cgi?action=URLList&event=14&end=999",
    "https://manbow.nothing.sh/event/event.cgi?action=URLList&event=15&end=999",
]
EOF

# 运行程序
cargo run -- --input urls.toml --output combined_data.toml --log-level info
```

### 调试模式

```bash
cargo run -- --log-level debug
```

这将显示详细的解析过程和调试信息。

## 依赖项

- `scraper`: HTML解析
- `toml`: TOML格式支持
- `surf`: HTTP客户端
- `clap`: 命令行参数解析
- `log` + `env_logger`: 日志系统
- `anyhow`: 错误处理
- `encoding_rs`: 字符编码检测
- `regex`: 正则表达式支持
