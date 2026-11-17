use tauri::ipc::Channel;
use tauri::Manager;
use serde::Serialize;
use std::io::{Seek, SeekFrom, Write};
use std::time::Duration;

#[derive(Serialize, Clone)]
struct DownloadProgress {
    downloaded: u64,
    total: u64,
}

/// 应用内下载安装包（同小蛋仓）：流式写入 + Range 断点续传 + 进度回调。
#[tauri::command]
async fn download_update(
    app: tauri::AppHandle,
    url: String,
    filename: String,
    on_progress: Channel<DownloadProgress>,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| format!("初始化网络失败：{}", e))?;

    let base = app
        .path()
        .download_dir()
        .ok()
        .or_else(|| app.path().app_data_dir().ok())
        .ok_or("无法获取可写目录")?;

    let safe_name = std::path::Path::new(&filename)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "update-installer".to_string());
    let dest = base.join(safe_name);

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(&dest)
        .map_err(|e| format!("创建文件失败：{}", e))?;
    let mut downloaded: u64 = std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
    let mut total: u64 = 0;
    let mut last_err = String::from("下载未完成");
    let max_attempts: u32 = 12;

    use futures_util::StreamExt;
    for _attempt in 0..max_attempts {
        if total > 0 && downloaded >= total {
            break;
        }
        let mut req = client
            .get(&url)
            .header("User-Agent", "yinji-app")
            .header("Accept-Encoding", "identity");
        if downloaded > 0 {
            req = req.header("Range", format!("bytes={}-", downloaded));
        }
        let resp = match req.send().await {
            Ok(r) => r,
            Err(e) => {
                last_err = format!("请求失败：{}", e);
                continue;
            }
        };
        let code = resp.status().as_u16();
        if code == 416 {
            return Ok(dest.to_string_lossy().to_string());
        }
        if code != 200 && code != 206 {
            last_err = format!("下载失败，HTTP {}", code);
            continue;
        }
        if code == 200 && downloaded > 0 {
            file.set_len(0).map_err(|e| format!("重置残档失败：{}", e))?;
            downloaded = 0;
            total = 0;
        }
        if total == 0 {
            let len = resp.content_length().unwrap_or(0);
            total = if code == 206 { downloaded.saturating_add(len) } else { len };
        }
        file.seek(SeekFrom::Start(downloaded))
            .map_err(|e| format!("定位文件失败：{}", e))?;

        let mut stream = resp.bytes_stream();
        let mut broken = false;
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(bytes) => {
                    file.write_all(&bytes[..]).map_err(|e| format!("写入失败：{}", e))?;
                    downloaded += bytes.len() as u64;
                    let _ = on_progress.send(DownloadProgress { downloaded, total });
                }
                Err(e) => {
                    last_err = format!("连接中断：{}", e);
                    broken = true;
                    break;
                }
            }
        }
        let _ = file.flush();
        let _ = on_progress.send(DownloadProgress { downloaded, total });
        if !broken && (total == 0 || downloaded >= total) {
            return Ok(dest.to_string_lossy().to_string());
        }
        if !broken {
            last_err = format!("连接提前结束（{}/{} 字节）", downloaded, total);
        }
    }
    Err(format!(
        "{}（已保留 {} 字节进度，点「继续下载」可从断点续传）",
        last_err, downloaded
    ))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![download_update])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
