use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use clap::Parser;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};

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

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// 事件文件路径 (例如: events/BOFTT.toml)
    #[arg(short, long)]
    event: PathBuf,

    /// 输出目录
    #[arg(short, long, default_value = "downloads")]
    output: PathBuf,

    /// 下载特定作品编号 (例如: 1,2,3)
    #[arg(long)]
    entries: Option<String>,

    /// 交互模式 - 为每个作品选择下载链接
    #[arg(long)]
    interactive: bool,

    /// 日志级别 (trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    log_level: String,
}

#[derive(Debug, Clone)]
enum LinkType {
    Direct {
        url: String,
    },
    GoogleDrive {
        share_id: String,
    },
    Dropbox {
        share_id: String,
    },
    OneDrive {
        url: String,
    },
    MediaFire {
        url: String,
    },
    Mega {
        #[allow(dead_code)]
        url: String,
    },
    Unknown {
        #[allow(dead_code)]
        url: String,
    },
}

impl LinkType {
    fn from_url(url: &str) -> Self {
        // 检查是否是纯分享ID格式（不包含协议）
        if url.len() > 20
            && url
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
            && !url.contains("://")
        {
            // 可能是分享ID，根据长度和字符特征判断
            if url.len() >= 20 && url.len() <= 50 {
                if url.chars().any(|c| c.is_uppercase()) {
                    // 包含大写字母，可能是Google Drive ID
                    Self::GoogleDrive {
                        share_id: url.to_string(),
                    }
                } else {
                    // 可能是Dropbox ID
                    Self::Dropbox {
                        share_id: url.to_string(),
                    }
                }
            } else {
                Self::Unknown {
                    url: url.to_string(),
                }
            }
        } else if url.starts_with("https://drive.google.com/")
            || url.starts_with("https://drive.usercontent.google.com/")
        {
            // 从Google Drive链接中提取ID
            if let Some(id) = Self::extract_google_drive_id(url) {
                Self::GoogleDrive { share_id: id }
            } else {
                Self::GoogleDrive {
                    share_id: url.to_string(),
                }
            }
        } else if url.starts_with("https://www.dropbox.com/")
            || url.starts_with("https://dl.dropboxusercontent.com/")
        {
            // 从Dropbox链接中提取ID
            if let Some(id) = Self::extract_dropbox_id(url) {
                Self::Dropbox { share_id: id }
            } else {
                Self::Dropbox {
                    share_id: url.to_string(),
                }
            }
        } else if url.starts_with("https://1drv.ms/") {
            Self::OneDrive {
                url: url.to_string(),
            }
        } else if url.starts_with("https://www.mediafire.com/") {
            Self::MediaFire {
                url: url.to_string(),
            }
        } else if url.starts_with("https://mega.nz/") || url.starts_with("https://mega.co.nz/") {
            Self::Mega {
                url: url.to_string(),
            }
        } else if url.starts_with("http://") || url.starts_with("https://") {
            Self::Direct {
                url: url.to_string(),
            }
        } else {
            Self::Unknown {
                url: url.to_string(),
            }
        }
    }

    fn extract_google_drive_id(url: &str) -> Option<String> {
        // 匹配格式: https://drive.google.com/file/d/ID/view
        if let Some(start) = url.find("/file/d/") {
            let id_start = start + 8;
            if let Some(end) = url[id_start..].find("/") {
                return Some(url[id_start..id_start + end].to_string());
            }
        }

        // 匹配格式: https://drive.google.com/uc?id=ID
        if let Some(start) = url.find("?id=") {
            let id_start = start + 4;
            if let Some(end) = url[id_start..].find("&") {
                return Some(url[id_start..id_start + end].to_string());
            } else {
                return Some(url[id_start..].to_string());
            }
        }

        // 匹配格式: https://drive.usercontent.google.com/download?id=ID
        if let Some(start) = url.find("/download?id=") {
            let id_start = start + 13;
            if let Some(end) = url[id_start..].find("&") {
                return Some(url[id_start..id_start + end].to_string());
            } else {
                return Some(url[id_start..].to_string());
            }
        }

        // 匹配格式: https://drive.usercontent.google.com/u/0/uc?id=ID
        if let Some(start) = url.find("/uc?id=") {
            let id_start = start + 7;
            if let Some(end) = url[id_start..].find("&") {
                return Some(url[id_start..id_start + end].to_string());
            } else {
                return Some(url[id_start..].to_string());
            }
        }

        None
    }

    fn extract_dropbox_id(url: &str) -> Option<String> {
        // 匹配格式: https://www.dropbox.com/s/ID/filename
        if let Some(start) = url.find("/s/") {
            let id_start = start + 3;
            if let Some(end) = url[id_start..].find("/") {
                return Some(url[id_start..id_start + end].to_string());
            }
        }

        // 匹配格式: https://www.dropbox.com/scl/fi/ID/filename
        // 匹配格式: https://www.dropbox.com/scl/fo/ID/filename
        // 匹配格式: https://dl.dropboxusercontent.com/scl/fi/ID/filename
        for pattern in ["/scl/fi/", "/scl/fo/"] {
            if let Some(start) = url.find(pattern) {
                let id_start = start + pattern.len();
                if let Some(end) = url[id_start..].find("/") {
                    return Some(url[id_start..id_start + end].to_string());
                }
            }
        }

        // 特殊处理 dropboxusercontent.com 格式
        if url.starts_with("https://dl.dropboxusercontent.com/scl/fi/") {
            let id_start = 40; // "https://dl.dropboxusercontent.com/scl/fi/".len()
            if let Some(end) = url[id_start..].find("/") {
                return Some(url[id_start..id_start + end].to_string());
            }
        }

        None
    }

    fn is_downloadable(&self) -> bool {
        match self {
            Self::Direct { .. }
            | Self::GoogleDrive { .. }
            | Self::Dropbox { .. }
            | Self::OneDrive { .. }
            | Self::MediaFire { .. } => true,
            Self::Mega { .. } | Self::Unknown { .. } => false,
        }
    }

    fn get_direct_url(&self) -> Option<String> {
        match self {
            Self::Direct { url } => Some(url.clone()),
            Self::GoogleDrive { share_id } => {
                // 如果是分享ID，直接构造下载链接
                if !share_id.contains("://") {
                    Some(format!(
                        "https://drive.google.com/uc?export=download&id={}",
                        share_id
                    ))
                } else {
                    // 如果是完整URL，尝试提取ID
                    if share_id.contains("/file/d/") && share_id.contains("/view") {
                        if let Some(id_start) = share_id.find("/file/d/") {
                            let id_start = id_start + 8;
                            if let Some(id_end) = share_id[id_start..].find("/") {
                                let id = &share_id[id_start..id_start + id_end];
                                return Some(format!(
                                    "https://drive.google.com/uc?export=download&id={}",
                                    id
                                ));
                            }
                        }
                    }
                    Some(share_id.clone())
                }
            }
            Self::Dropbox { share_id } => {
                // 如果是分享ID，构造下载链接
                if !share_id.contains("://") {
                    Some(format!("https://www.dropbox.com/s/{}/file?dl=1", share_id))
                } else {
                    // 如果是完整URL，尝试转换为直接下载链接
                    if share_id.contains("?dl=0") {
                        Some(share_id.replace("?dl=0", "?dl=1"))
                    } else if !share_id.contains("?dl=") {
                        Some(format!("{}&dl=1", share_id))
                    } else {
                        Some(share_id.clone())
                    }
                }
            }
            Self::OneDrive { url } => Some(url.clone()),
            Self::MediaFire { url } => Some(url.clone()),
            _ => None,
        }
    }
}

fn load_event_data(path: &Path) -> Result<BmsData> {
    info!("加载事件文件: {:?}", path);
    let content = fs::read_to_string(path).with_context(|| format!("无法读取文件: {:?}", path))?;

    let data: BmsData =
        toml::from_str(&content).with_context(|| format!("解析TOML文件失败: {:?}", path))?;

    info!("加载了 {} 个作品", data.entries.len());
    Ok(data)
}

fn filter_entries<'a>(
    data: &'a BmsData,
    entries_filter: Option<&'a str>,
) -> Result<Vec<&'a BmsEntry>> {
    let entries = if let Some(filter) = entries_filter {
        let numbers: Vec<&str> = filter.split(',').map(|s| s.trim()).collect();
        let mut filtered = Vec::new();

        for entry in &data.entries {
            if numbers.contains(&entry.no.as_str()) {
                filtered.push(entry);
            }
        }

        if filtered.is_empty() {
            warn!("没有找到编号为 {} 的作品", filter);
        }
        filtered
    } else {
        data.entries.iter().collect()
    };

    Ok(entries)
}

fn analyze_links(entry: &BmsEntry) -> (Vec<LinkType>, Vec<String>) {
    let mut links = Vec::new();
    let mut non_links = Vec::new();

    for addr in &entry.addr {
        // 检查是否是分享ID格式（20-50个字符，只包含字母数字、连字符、下划线）
        if addr.len() >= 20
            && addr.len() <= 50
            && addr
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
            && !addr.contains("://")
        {
            links.push(LinkType::from_url(addr));
        } else if addr.starts_with("http://") || addr.starts_with("https://") {
            links.push(LinkType::from_url(addr));
        } else {
            non_links.push(addr.clone());
        }
    }

    (links, non_links)
}

fn select_download_link(entry: &BmsEntry, interactive: bool) -> Result<Option<String>> {
    let (links, non_links) = analyze_links(entry);

    if links.is_empty() {
        warn!("作品 #{} - {} 没有可下载的链接", entry.no, entry.title);
        if !non_links.is_empty() {
            info!("  非链接内容: {}", non_links.join(", "));
        }
        return Ok(None);
    }

    let downloadable_links: Vec<_> = links.iter().filter(|link| link.is_downloadable()).collect();

    if downloadable_links.is_empty() {
        warn!("作品 #{} - {} 没有支持的下载链接", entry.no, entry.title);
        info!(
            "  可用链接: {}",
            links
                .iter()
                .map(|l| format!("{:?}", l))
                .collect::<Vec<_>>()
                .join(", ")
        );
        return Ok(None);
    }

    if downloadable_links.len() == 1 {
        // 只有一个可下载链接，直接使用
        let link = downloadable_links[0].get_direct_url().unwrap();
        info!(
            "作品 #{} - {} 使用唯一链接: {}",
            entry.no, entry.title, link
        );
        return Ok(Some(link));
    }

    // 多个链接的情况
    if interactive {
        println!("\n作品 #{} - {}", entry.no, entry.title);
        println!("作者: {}", entry.name);
        if let Some(team) = &entry.team {
            println!("团队: {}", team);
        }
        println!("大小: {}", entry.size);
        println!("\n可用的下载链接:");

        for (i, link) in downloadable_links.iter().enumerate() {
            let direct_url = link
                .get_direct_url()
                .unwrap_or_else(|| "无法获取直接链接".to_string());
            println!("  {}. {} -> {}", i + 1, format!("{:?}", link), direct_url);
        }

        println!("\n请选择要下载的链接 (输入数字，或按 Enter 跳过):");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if let Ok(choice) = input.trim().parse::<usize>() {
            if choice > 0 && choice <= downloadable_links.len() {
                let selected_link = downloadable_links[choice - 1].get_direct_url().unwrap();
                return Ok(Some(selected_link));
            }
        }

        info!("跳过作品 #{} - {}", entry.no, entry.title);
        return Ok(None);
    } else {
        // 非交互模式，提示用户
        warn!(
            "作品 #{} - {} 有多个下载链接，请使用 --interactive 模式选择:",
            entry.no, entry.title
        );
        for (i, link) in downloadable_links.iter().enumerate() {
            let direct_url = link
                .get_direct_url()
                .unwrap_or_else(|| "无法获取直接链接".to_string());
            println!("  {}. {} -> {}", i + 1, format!("{:?}", link), direct_url);
        }
        return Ok(None);
    }
}

async fn download_file(url: &str, output_path: &Path) -> Result<()> {
    info!("下载: {} -> {:?}", url, output_path);

    // 确保输出目录存在
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut response = surf::get(url)
        .await
        .map_err(|e| anyhow::anyhow!("HTTP请求失败: {} - {}", url, e))?;

    let mut file = fs::File::create(output_path)
        .with_context(|| format!("创建文件失败: {:?}", output_path))?;

    let bytes = response
        .body_bytes()
        .await
        .map_err(|e| anyhow::anyhow!("读取响应失败: {} - {}", url, e))?;

    std::io::Write::write_all(&mut file, &bytes)
        .with_context(|| format!("写入文件失败: {:?}", output_path))?;

    info!("下载完成: {:?}", output_path);
    Ok(())
}

fn generate_filename(entry: &BmsEntry) -> String {
    let mut filename = format!("{} - {}", entry.no, entry.title);

    // 清理文件名中的非法字符
    filename = filename
        .replace("/", "_")
        .replace("\\", "_")
        .replace(":", "_")
        .replace("*", "_")
        .replace("?", "_")
        .replace("\"", "_")
        .replace("<", "_")
        .replace(">", "_")
        .replace("|", "_");

    // 如果文件名太长，截断
    if filename.len() > 100 {
        filename = format!("{}...", &filename[..97]);
    }

    filename
}

async fn download_entry(entry: &BmsEntry, output_dir: &Path, interactive: bool) -> Result<()> {
    let filename = generate_filename(entry);
    let output_path = output_dir.join(&filename);

    if let Some(download_url) = select_download_link(entry, interactive)? {
        download_file(&download_url, &output_path).await?;
    }

    Ok(())
}

async fn async_main(args: Args) -> Result<()> {
    // 加载事件数据
    let data = load_event_data(&args.event)?;

    // 过滤作品
    let entries = filter_entries(&data, args.entries.as_deref())?;

    if entries.is_empty() {
        error!("没有找到要下载的作品");
        return Ok(());
    }

    // 创建输出目录
    fs::create_dir_all(&args.output)?;

    info!("开始下载 {} 个作品到 {:?}", entries.len(), args.output);

    // 下载每个作品
    for entry in entries {
        if let Err(e) = download_entry(entry, &args.output, args.interactive).await {
            error!("下载作品 #{} 失败: {}", entry.no, e);
        }
    }

    info!("下载完成！");
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
