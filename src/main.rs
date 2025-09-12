use std::{
    collections::HashMap,
    io::{self, Read},
    path::PathBuf,
};

use anyhow::Result;
use clap::Parser;
use encoding_rs::{EUC_JP, SHIFT_JIS, UTF_8};
use log::{debug, error, info};
use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};
use surf;

#[derive(Debug, Serialize, Deserialize)]
struct BmsEntry {
    no: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    team: Option<String>,
    title: String,
    size: String,
    addr: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BmsData {
    entries: Vec<BmsEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct UrlConfig {
    urls: Vec<String>,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// 输入TOML配置文件路径，包含要抓取的URL列表
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// 输出文件路径，如果不指定则输出到stdout
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// 从stdin读取URL列表（每行一个URL）
    #[arg(long)]
    stdin: bool,

    /// 日志级别 (trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    log_level: String,
}

#[derive(Debug, Clone)]
struct ColumnMapping {
    no: Option<usize>,
    name: Option<usize>,
    team: Option<usize>,
    title: Option<usize>,
    size: Option<usize>,
    addr: Option<usize>,
}

fn clean_html_text(element: ElementRef) -> String {
    element
        .text()
        .collect::<Vec<_>>()
        .join("")
        .trim()
        .to_string()
}

fn detect_column_mapping(document: &Html) -> Result<ColumnMapping> {
    let row_selector = Selector::parse("tr").unwrap();
    let cell_selector = Selector::parse("td, th").unwrap();

    // 尝试找到表格头部
    for row in document.select(&row_selector) {
        let cells: Vec<_> = row.select(&cell_selector).collect();

        if cells.is_empty() {
            continue;
        }

        // 获取所有单元格的文本内容
        let cell_texts: Vec<String> = cells
            .iter()
            .map(|cell| clean_html_text(*cell).to_lowercase())
            .collect();

        // 检查是否看起来像表头（包含已知的列名）
        let has_header_indicators = cell_texts.iter().any(|text| {
            text.contains("no")
                || text.contains("name")
                || text.contains("title")
                || text.contains("size")
                || text.contains("addr")
                || text.contains("team")
                || text == "no"
                || text == "name"
                || text == "title"
                || text == "size"
                || text == "addr"
                || text == "team"
        });

        if !has_header_indicators {
            continue;
        }

        // 检测到表格头部

        let mut mapping = ColumnMapping {
            no: None,
            name: None,
            team: None,
            title: None,
            size: None,
            addr: None,
        };

        // 分析每一列的内容来确定其用途
        for (idx, text) in cell_texts.iter().enumerate() {
            match text.as_str() {
                text if text.contains("no") => mapping.no = Some(idx),
                text if text.contains("name") => mapping.name = Some(idx),
                text if text.contains("team") => mapping.team = Some(idx),
                text if text.contains("title") => mapping.title = Some(idx),
                text if text.contains("size") => mapping.size = Some(idx),
                text if text.contains("addr") => mapping.addr = Some(idx),
                _ => {}
            }
        }

        // 如果没有找到明确的列名，尝试根据位置和内容推断
        if mapping.no.is_none() && !cell_texts.is_empty() {
            // 第一列通常是序号
            mapping.no = Some(0);
        }

        if mapping.name.is_none() && cell_texts.len() > 1 {
            // 第二列通常是名称
            mapping.name = Some(1);
        }

        // 根据列数推断其他字段的位置
        match cell_texts.len() {
            5 => {
                // 格式: No, Name, Title, Size, Addr
                if mapping.title.is_none() {
                    mapping.title = Some(2);
                }
                if mapping.size.is_none() {
                    mapping.size = Some(3);
                }
                if mapping.addr.is_none() {
                    mapping.addr = Some(4);
                }
            }
            6 => {
                // 格式: No, Name, Team, Title, Size, Addr
                if mapping.team.is_none() {
                    mapping.team = Some(2);
                }
                if mapping.title.is_none() {
                    mapping.title = Some(3);
                }
                if mapping.size.is_none() {
                    mapping.size = Some(4);
                }
                if mapping.addr.is_none() {
                    mapping.addr = Some(5);
                }
            }
            _ => {
                // 尝试从右往左推断：最后一列是Addr，倒数第二列是Size
                if mapping.addr.is_none() && cell_texts.len() > 0 {
                    mapping.addr = Some(cell_texts.len() - 1);
                }
                if mapping.size.is_none() && cell_texts.len() > 1 {
                    mapping.size = Some(cell_texts.len() - 2);
                }
                if mapping.title.is_none() && cell_texts.len() > 2 {
                    mapping.title = Some(cell_texts.len() - 3);
                }
                if mapping.team.is_none() && cell_texts.len() > 3 {
                    mapping.team = Some(cell_texts.len() - 4);
                }
            }
        }

        // 列映射已建立
        return Ok(mapping);
    }

    // 如果没有找到明确的表头，返回默认映射（6列格式）
    // 未找到表头，使用默认6列映射
    Ok(ColumnMapping {
        no: Some(0),
        name: Some(1),
        team: Some(2),
        title: Some(3),
        size: Some(4),
        addr: Some(5),
    })
}

fn clean_html_content(html_content: &str) -> String {
    // 清理HTML实体
    let content = html_content
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ");

    // 移除HTML标签
    let tag_regex = Regex::new(r"<[^>]*>").unwrap();
    let clean_text = tag_regex.replace_all(&content, "");

    clean_text.trim().to_string()
}

fn extract_urls_and_text(input: &str) -> Vec<String> {
    let cleaned_input = clean_html_content(input);
    // 改进的正则表达式：只匹配URL中允许出现的字符（基于RFC 3986标准）
    // 安全字符：字母数字、连字符、下划线、点号、波浪线
    // 保留字符：冒号、斜杠、问号、井号、等号、与号、百分号
    let url_regex = Regex::new(r"https?://[a-zA-Z0-9\-._~:/?#=&%]+").unwrap();
    let mut result = Vec::new();
    let mut last_end = 0;

    for mat in url_regex.find_iter(&cleaned_input) {
        // 添加URL前的文本（如果有的话）
        if mat.start() > last_end {
            let text = cleaned_input[last_end..mat.start()].trim();
            if !text.is_empty() {
                result.push(text.to_string());
            }
        }

        // 添加URL
        result.push(mat.as_str().to_string());
        last_end = mat.end();
    }

    // 添加最后剩余的文本
    if last_end < cleaned_input.len() {
        let text = cleaned_input[last_end..].trim();
        if !text.is_empty() {
            result.push(text.to_string());
        }
    }

    // 如果没有找到URL，返回原始文本（如果非空）
    if result.is_empty() && !cleaned_input.trim().is_empty() {
        result.push(cleaned_input);
    }

    result
}

fn detect_and_decode_content(bytes: &[u8]) -> String {
    // 尝试不同的编码
    let encodings = [UTF_8, SHIFT_JIS, EUC_JP];

    for encoding in &encodings {
        let (decoded, _, false) = encoding.decode(bytes) else {
            continue;
        };
        // 检查是否包含有效的日文字符或ASCII
        if is_valid_content(&decoded) {
            return decoded.into_owned();
        }
    }

    // 如果所有编码都失败，使用UTF-8并替换错误字符
    let (decoded, _, _) = UTF_8.decode(bytes);
    decoded.into_owned()
}

fn is_valid_content(text: &str) -> bool {
    // 检查文本是否包含合理的字符
    let valid_chars = text
        .chars()
        .filter(|c| {
            c.is_ascii() ||
            (*c >= '\u{3040}' && *c <= '\u{309F}') || // 平假名
            (*c >= '\u{30A0}' && *c <= '\u{30FF}') || // 片假名
            (*c >= '\u{4E00}' && *c <= '\u{9FAF}') // 汉字
        })
        .count();

    valid_chars as f64 / text.len() as f64 > 0.7
}

async fn fetch_and_parse_table(url: &str) -> Result<BmsData> {
    debug!("正在获取网页内容: {}", url);
    let mut response = surf::get(url)
        .await
        .map_err(|e| anyhow::anyhow!("HTTP请求失败: {}", e))?;
    let response_bytes = response
        .body_bytes()
        .await
        .map_err(|e| anyhow::anyhow!("读取响应失败: {}", e))?;

    // 尝试检测并正确解码内容
    let html_content = detect_and_decode_content(&response_bytes);

    debug!("正在解析HTML...");
    let document = Html::parse_document(&html_content);

    // 检测列映射
    let column_mapping = detect_column_mapping(&document)?;

    // 选择表格行
    let row_selector = Selector::parse("tr").unwrap();
    let cell_selector = Selector::parse("td").unwrap();

    let mut entries = Vec::new();
    let mut seen_entries = HashMap::new(); // 用于去重

    for row in document.select(&row_selector) {
        let cells: Vec<_> = row.select(&cell_selector).collect();

        // 确保行有足够的列
        if cells.is_empty() {
            continue;
        }

        // 使用列映射获取各字段的值
        let no_text = if let Some(idx) = column_mapping.no {
            if idx < cells.len() {
                clean_html_text(cells[idx])
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let name_text = if let Some(idx) = column_mapping.name {
            if idx < cells.len() {
                clean_html_text(cells[idx])
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        // 跳过表头或空行
        if no_text.is_empty() || no_text.to_lowercase() == "no" || name_text.is_empty() {
            continue;
        }

        let team_text = if let Some(idx) = column_mapping.team {
            if idx < cells.len() {
                let team = clean_html_text(cells[idx]);
                if team.is_empty() { None } else { Some(team) }
            } else {
                None
            }
        } else {
            None
        };

        let title_text = if let Some(idx) = column_mapping.title {
            if idx < cells.len() {
                clean_html_text(cells[idx])
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let size_text = if let Some(idx) = column_mapping.size {
            if idx < cells.len() {
                clean_html_text(cells[idx])
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let addr_html = if let Some(idx) = column_mapping.addr {
            if idx < cells.len() {
                cells[idx].inner_html()
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        // 从链接中提取序号（如果存在）
        let no_clean = if no_text.parse::<u32>().is_ok() {
            no_text
        } else {
            // 尝试从HTML中提取数字
            let num_regex = Regex::new(r"\d+").unwrap();
            if let Some(no_idx) = column_mapping.no {
                if no_idx < cells.len() {
                    if let Some(mat) = num_regex.find(&cells[no_idx].inner_html()) {
                        mat.as_str().to_string()
                    } else {
                        no_text
                    }
                } else {
                    no_text
                }
            } else {
                no_text
            }
        };

        // 创建唯一键用于去重
        let unique_key = format!("{}|{}|{}", no_clean, name_text, title_text);
        if seen_entries.contains_key(&unique_key) {
            continue;
        }

        // 处理地址字段 - 按换行符分割，然后提取URL
        let addr_lines: Vec<String> = addr_html
            .split("<br>")
            .flat_map(|line| extract_urls_and_text(line))
            .filter(|s| !s.trim().is_empty())
            .collect();

        let entry = BmsEntry {
            no: no_clean,
            name: name_text,
            team: team_text,
            title: title_text,
            size: size_text,
            addr: addr_lines,
        };

        entries.push(entry);
        seen_entries.insert(unique_key, true);
    }

    debug!("解析完成，找到 {} 个条目（去重后）", entries.len());

    Ok(BmsData { entries })
}

fn convert_to_toml(data: &BmsData) -> Result<String> {
    debug!("正在转换为TOML格式...");
    let toml_string = toml::to_string_pretty(data)?;
    Ok(toml_string)
}

fn read_urls_from_stdin() -> Result<Vec<String>> {
    debug!("从stdin读取URL列表...");
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let urls: Vec<String> = input
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty() && line.starts_with("http"))
        .collect();

    debug!("从stdin读取到 {} 个URL", urls.len());
    Ok(urls)
}

fn read_urls_from_file(path: &PathBuf) -> Result<Vec<String>> {
    debug!("从文件读取URL配置: {:?}", path);
    let content = std::fs::read_to_string(path)?;
    let config: UrlConfig = toml::from_str(&content)?;
    debug!("从文件读取到 {} 个URL", config.urls.len());
    Ok(config.urls)
}

fn write_output(content: &str, output_path: &Option<PathBuf>) -> Result<()> {
    match output_path {
        Some(path) => {
            debug!("写入输出到文件: {:?}", path);
            std::fs::write(path, content)?;
            info!("数据已保存到文件: {:?}", path);
        }
        None => {
            debug!("输出到stdout");
            print!("{}", content);
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    // 初始化日志
    env_logger::Builder::from_default_env()
        .filter_level(match args.log_level.as_str() {
            "trace" => log::LevelFilter::Trace,
            "debug" => log::LevelFilter::Debug,
            "info" => log::LevelFilter::Info,
            "warn" => log::LevelFilter::Warn,
            "error" => log::LevelFilter::Error,
            _ => log::LevelFilter::Info,
        })
        .init();

    smol::block_on(async_main(args))
}

async fn async_main(args: Args) -> Result<()> {
    // 获取URL列表
    let urls = if args.stdin {
        read_urls_from_stdin()?
    } else if let Some(input_path) = &args.input {
        read_urls_from_file(input_path)?
    } else {
        // 默认URL
        vec![
            "https://manbow.nothing.sh/event/event.cgi?action=URLList&event=146&end=999"
                .to_string(),
        ]
    };

    if urls.is_empty() {
        error!("没有找到任何URL");
        return Ok(());
    }

    info!("开始处理 {} 个URL", urls.len());

    let mut all_entries = Vec::new();

    for (i, url) in urls.iter().enumerate() {
        info!("处理URL {}/{}: {}", i + 1, urls.len(), url);

        match fetch_and_parse_table(url).await {
            Ok(data) => {
                info!("成功解析URL: {} ({} 个条目)", url, data.entries.len());
                all_entries.extend(data.entries);
            }
            Err(e) => {
                error!("处理URL失败 {}: {}", url, e);
            }
        }
    }

    if all_entries.is_empty() {
        error!("没有成功解析任何数据");
        return Ok(());
    }

    info!("总共收集到 {} 个条目", all_entries.len());

    let bms_data = BmsData {
        entries: all_entries,
    };

    match convert_to_toml(&bms_data) {
        Ok(toml_output) => {
            write_output(&toml_output, &args.output)?;
        }
        Err(e) => {
            error!("转换为TOML时出错: {}", e);
        }
    }

    Ok(())
}
