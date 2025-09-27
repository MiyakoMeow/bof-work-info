use std::{
    fs,
    io::{BufReader, Read},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct BmsEntry {
    pub no: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team: Option<String>,
    pub title: String,
    pub size: String,
    pub addr: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum LinkType {
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
    pub fn from_url(url: &str) -> Self {
        // 只处理完整的URL，不再支持纯分享ID
        if url.starts_with("https://drive.google.com/")
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

    pub fn extract_google_drive_id(url: &str) -> Option<String> {
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

    pub fn extract_dropbox_id(url: &str) -> Option<String> {
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

    pub fn is_downloadable(&self) -> bool {
        match self {
            Self::Direct { .. }
            | Self::GoogleDrive { .. }
            | Self::Dropbox { .. }
            | Self::OneDrive { .. }
            | Self::MediaFire { .. } => true,
            Self::Mega { .. } | Self::Unknown { .. } => false,
        }
    }

    pub fn get_direct_url(&self) -> Option<String> {
        match self {
            Self::Direct { url } => Some(url.clone()),
            Self::GoogleDrive { share_id } => {
                // 如果share_id是完整URL，直接返回
                if share_id.starts_with("https://") {
                    Some(share_id.clone())
                } else {
                    // 如果只是ID，构造下载链接
                    Some(format!(
                        "https://drive.google.com/uc?export=download&id={}",
                        share_id
                    ))
                }
            }
            Self::Dropbox { share_id } => {
                // 处理完整URL，尝试转换为直接下载链接
                if share_id.contains("?dl=0") {
                    Some(share_id.replace("?dl=0", "?dl=1"))
                } else if !share_id.contains("?dl=") {
                    Some(format!("{}&dl=1", share_id))
                } else {
                    Some(share_id.clone())
                }
            }
            Self::OneDrive { url } => Some(url.clone()),
            Self::MediaFire { url } => Some(url.clone()),
            _ => None,
        }
    }
}

pub fn is_valid_url(url: &str) -> bool {
    // 基本URL格式验证
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return false;
    }

    // 检查是否包含有效的域名部分
    if let Some(colon_pos) = url.find("://") {
        let after_protocol = &url[colon_pos + 3..];
        if after_protocol.is_empty() || !after_protocol.contains('.') {
            return false;
        }
    } else {
        return false;
    }

    // 可以添加更多URL格式验证规则
    true
}

pub fn analyze_links(entry: &BmsEntry) -> (Vec<LinkType>, Vec<String>) {
    let mut links = Vec::new();
    let mut non_links = Vec::new();

    for addr in &entry.addr {
        // 只接受以 http:// 或 https:// 开头的有效URL
        if addr.starts_with("http://") || addr.starts_with("https://") {
            // 进一步验证URL格式
            if is_valid_url(addr) {
                links.push(LinkType::from_url(addr));
            } else {
                non_links.push(addr.clone());
            }
        } else {
            non_links.push(addr.clone());
        }
    }

    (links, non_links)
}

pub fn select_download_link(entry: &BmsEntry, interactive: bool) -> Result<Option<String>> {
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

pub async fn download_google_drive_file(file_id: &str, output_path: &Path) -> Result<PathBuf> {
    info!("下载Google Drive文件: {} -> {:?}", file_id, output_path);

    // 确保输出目录存在
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // 创建HTTP客户端
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| anyhow::anyhow!("创建HTTP客户端失败: {}", e))?;

    // 第一步：获取cookie和确认页面
    let cookie_path = output_path.parent().unwrap().join("cookie");
    let cookie_url = format!("https://drive.google.com/uc?export=download&id={}", file_id);

    info!("第一步：获取cookie，URL: {}", cookie_url);

    let cookie_response = client
        .get(&cookie_url)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("获取cookie失败: {} - {}", cookie_url, e))?;

    // 保存cookie内容
    let cookie_content = cookie_response
        .text()
        .await
        .map_err(|e| anyhow::anyhow!("读取cookie响应失败: {} - {}", cookie_url, e))?;

    info!("Cookie内容长度: {} 字节", cookie_content.len());

    fs::write(&cookie_path, &cookie_content)
        .with_context(|| format!("保存cookie失败: {:?}", cookie_path))?;

    // 从HTML内容中提取下载URL和文件名
    let download_url = if let Some(url) = extract_download_url_from_html(&cookie_content) {
        info!("从HTML中提取到下载URL: {}", url);
        url
    } else {
        // 如果没有找到完整的下载URL，尝试使用确认token
        let confirm_token = extract_confirm_token_from_html(&cookie_content)?;
        info!("提取到的确认token: '{}'", confirm_token);

        if confirm_token.is_empty() {
            info!("没有找到确认token，直接使用原始URL");
            cookie_url.clone()
        } else {
            let url = format!(
                "https://drive.google.com/uc?export=download&confirm={}&id={}",
                confirm_token, file_id
            );
            info!("使用确认token构造下载URL: {}", url);
            url
        }
    };

    // 尝试从HTML中提取原始文件名
    let original_filename = extract_filename_from_html(&cookie_content);

    info!("第二步：下载文件，URL: {}", download_url);

    let response = client
        .get(&download_url)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("下载文件失败: {} - {}", download_url, e))?;

    // 检查响应状态
    let status = response.status();
    info!("HTTP响应状态: {}", status);

    // 尝试从Content-Disposition头中提取文件名
    let mut filename = None;
    if let Some(disposition) = response.headers().get("Content-Disposition") {
        let disposition_str = disposition.to_str().unwrap_or("");
        info!("Content-Disposition: {}", disposition_str);
        filename = extract_filename_from_disposition(disposition_str);
    }

    // 确定最终的文件名：优先使用Content-Disposition，然后是HTML中的文件名，最后是默认名称
    let final_output_path = if let Some(header_filename) = &filename {
        let parent_dir = output_path.parent().unwrap();
        let new_path = parent_dir.join(header_filename);
        info!(
            "使用Content-Disposition中的文件名: {} -> {:?}",
            header_filename, new_path
        );
        new_path
    } else if let Some(html_filename) = &original_filename {
        let parent_dir = output_path.parent().unwrap();
        let new_path = parent_dir.join(html_filename);
        info!("使用HTML中的文件名: {} -> {:?}", html_filename, new_path);
        new_path
    } else {
        output_path.to_path_buf()
    };

    let mut file = fs::File::create(&final_output_path)
        .with_context(|| format!("创建文件失败: {:?}", final_output_path))?;

    let bytes = response
        .bytes()
        .await
        .map_err(|e| anyhow::anyhow!("读取响应失败: {} - {}", download_url, e))?;

    info!("下载的数据长度: {} 字节", bytes.len());

    std::io::Write::write_all(&mut file, &bytes)
        .with_context(|| format!("写入文件失败: {:?}", final_output_path))?;

    // 清理cookie文件
    let _ = fs::remove_file(&cookie_path);

    info!(
        "Google Drive下载完成: {:?} ({} 字节)",
        final_output_path,
        bytes.len()
    );
    Ok(final_output_path)
}

pub fn extract_filename_from_disposition(disposition: &str) -> Option<String> {
    // 解析 Content-Disposition 头，格式如：
    // attachment; filename="filename.zip"
    // attachment; filename*=UTF-8''filename.zip
    if let Some(start) = disposition.find("filename=") {
        let filename_start = start + 9;
        let filename_part = &disposition[filename_start..];

        // 处理引号包围的文件名
        if filename_part.starts_with('"') {
            if let Some(end) = filename_part[1..].find('"') {
                return Some(filename_part[1..end + 1].to_string());
            }
        }
        // 处理没有引号的文件名
        else if let Some(end) = filename_part.find(';') {
            return Some(filename_part[..end].to_string());
        } else {
            return Some(filename_part.to_string());
        }
    }

    // 处理 filename*=UTF-8'' 格式
    if let Some(start) = disposition.find("filename*=UTF-8''") {
        let filename_start = start + 16;
        let filename_part = &disposition[filename_start..];
        if let Some(end) = filename_part.find(';') {
            return Some(filename_part[..end].to_string());
        } else {
            return Some(filename_part.to_string());
        }
    }

    None
}

pub fn extract_confirm_token_from_html(html_content: &str) -> Result<String> {
    // 在HTML中查找确认token
    // 通常格式为: <a href="/uc?export=download&confirm=TOKEN&id=FILE_ID"
    if let Some(start) = html_content.find("confirm=") {
        let token_start = start + 8;
        if let Some(end) = html_content[token_start..].find('&') {
            return Ok(html_content[token_start..token_start + end].to_string());
        } else if let Some(end) = html_content[token_start..].find('"') {
            return Ok(html_content[token_start..token_start + end].to_string());
        } else {
            return Ok(html_content[token_start..].to_string());
        }
    }

    // 也尝试查找其他可能的格式
    if let Some(start) = html_content.find("&confirm=") {
        let token_start = start + 9;
        if let Some(end) = html_content[token_start..].find('&') {
            return Ok(html_content[token_start..token_start + end].to_string());
        } else if let Some(end) = html_content[token_start..].find('"') {
            return Ok(html_content[token_start..token_start + end].to_string());
        } else {
            return Ok(html_content[token_start..].to_string());
        }
    }

    Ok(String::new())
}

pub fn extract_download_url_from_html(html_content: &str) -> Option<String> {
    // 查找隐藏字段的值
    let mut id = None;
    let mut export = None;
    let mut confirm = None;
    let mut uuid = None;

    // 提取id
    if let Some(start) = html_content.find("name=\"id\" value=\"") {
        let value_start = start + 17;
        if let Some(end) = html_content[value_start..].find('"') {
            id = Some(html_content[value_start..value_start + end].to_string());
        }
    }

    // 提取export
    if let Some(start) = html_content.find("name=\"export\" value=\"") {
        let value_start = start + 20;
        if let Some(end) = html_content[value_start..].find('"') {
            export = Some(html_content[value_start..value_start + end].to_string());
        }
    }

    // 提取confirm
    if let Some(start) = html_content.find("name=\"confirm\" value=\"") {
        let value_start = start + 21;
        if let Some(end) = html_content[value_start..].find('"') {
            confirm = Some(html_content[value_start..value_start + end].to_string());
        }
    }

    // 提取uuid
    if let Some(start) = html_content.find("name=\"uuid\" value=\"") {
        let value_start = start + 19;
        if let Some(end) = html_content[value_start..].find('"') {
            uuid = Some(html_content[value_start..value_start + end].to_string());
        }
    }

    info!(
        "提取到的参数: id={:?}, export={:?}, confirm={:?}, uuid={:?}",
        id, export, confirm, uuid
    );

    // 构造完整的下载URL
    if let (Some(id_val), Some(export_val), Some(confirm_val), Some(uuid_val)) =
        (id, export, confirm, uuid)
    {
        let url = format!(
            "https://drive.usercontent.google.com/download?id={}&export={}&confirm={}&uuid={}",
            id_val, export_val, confirm_val, uuid_val
        );
        info!("构造的下载URL: {}", url);
        return Some(url);
    }

    None
}

pub fn extract_filename_from_html(html_content: &str) -> Option<String> {
    // 从HTML中提取文件名，格式如: <a href="/open?id=...">filename.zip</a>
    if let Some(start) = html_content.find(">") {
        if let Some(end) = html_content[start + 1..].find("<") {
            let filename = &html_content[start + 1..start + 1 + end];
            if filename.contains('.') && !filename.contains(' ') {
                info!("从HTML中提取到文件名: {}", filename);
                return Some(filename.to_string());
            }
        }
    }

    None
}

pub fn is_valid_archive(file_path: &Path) -> Result<bool> {
    let file =
        fs::File::open(file_path).with_context(|| format!("无法打开文件: {:?}", file_path))?;

    let mut reader = BufReader::new(file);
    let mut header = [0u8; 4];

    if reader.read_exact(&mut header).is_err() {
        return Ok(false);
    }

    // 检查ZIP文件头 (PK)
    if header[0] == 0x50 && header[1] == 0x4B {
        return Ok(true);
    }

    // 检查RAR文件头
    if header[0] == 0x52 && header[1] == 0x61 && header[2] == 0x72 && header[3] == 0x21 {
        return Ok(true);
    }

    // 检查7Z文件头
    if header[0] == 0x37 && header[1] == 0x7A && header[2] == 0xBC && header[3] == 0xAF {
        return Ok(true);
    }

    // 检查TAR文件头 (需要更多字节，但先检查前4个)
    if header[0] == 0x75 && header[1] == 0x73 && header[2] == 0x74 && header[3] == 0x61 {
        return Ok(true);
    }

    Ok(false)
}

pub async fn download_file(url: &str, output_path: &Path) -> Result<()> {
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

pub fn generate_filename(entry: &BmsEntry) -> String {
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

pub async fn download_entry(entry: &BmsEntry, output_dir: &Path, interactive: bool) -> Result<()> {
    let filename = generate_filename(entry);
    let output_path = output_dir.join(&filename);

    if let Some(download_url) = select_download_link(entry, interactive)? {
        // 检查是否是Google Drive链接
        if download_url.contains("drive.google.com") {
            // 提取文件ID
            if let Some(file_id) = extract_google_drive_id_from_url(&download_url) {
                let final_path = download_google_drive_file(&file_id, &output_path).await?;

                // 验证下载的文件是否为有效压缩包
                if is_valid_archive(&final_path)? {
                    info!("文件验证成功: {:?} 是有效的压缩包", final_path);
                } else {
                    warn!("文件验证失败: {:?} 不是有效的压缩包", final_path);
                }
            } else {
                error!("无法从Google Drive URL中提取文件ID: {}", download_url);
            }
        } else {
            download_file(&download_url, &output_path).await?;
        }
    }

    Ok(())
}

pub fn extract_google_drive_id_from_url(url: &str) -> Option<String> {
    // 匹配格式: https://drive.google.com/file/d/ID/view
    if let Some(start) = url.find("/file/d/") {
        let id_start = start + 8;
        if let Some(end) = url[id_start..].find("/") {
            return Some(url[id_start..id_start + end].to_string());
        }
    }

    // 匹配格式: https://drive.google.com/uc?id=ID 或 https://drive.google.com/uc?export=download&id=ID
    if let Some(start) = url.find("?id=") {
        let id_start = start + 4;
        if let Some(end) = url[id_start..].find("&") {
            return Some(url[id_start..id_start + end].to_string());
        } else {
            return Some(url[id_start..].to_string());
        }
    }

    // 匹配格式: https://drive.google.com/uc?export=download&id=ID
    if let Some(start) = url.find("&id=") {
        let id_start = start + 4;
        if let Some(end) = url[id_start..].find("&") {
            return Some(url[id_start..id_start + end].to_string());
        } else {
            return Some(url[id_start..].to_string());
        }
    }

    None
}
