use anyhow::Result;
use std::{
    fs,
    path::{Path, PathBuf},
};
use tracing::info;

/// 获取文件大小
pub fn get_file_size(path: &Path) -> Result<u64> {
    let metadata = std::fs::metadata(path)?;
    Ok(metadata.len())
}

// /// 获取文件扩展名
// pub fn get_file_extension(path: &Path) -> Option<String> {
//     path.extension()
//         .and_then(|ext| ext.to_str())
//         .map(|ext| ext.to_lowercase())
// }

// /// 检查是否为视频文件
// pub fn is_video_file(path: &Path) -> bool {
//     if let Some(ext) = get_file_extension(path) {
//         matches!(
//             ext.as_str(),
//             "mp4" | "avi" | "mov" | "mkv" | "flv" | "wmv" | "webm" | "m4v"
//         )
//     } else {
//         false
//     }
// }

// /// 获取文件名（不含扩展名）
// pub fn get_file_stem(path: &Path) -> Option<String> {
//     path.file_stem()
//         .and_then(|stem| stem.to_str())
//         .map(|stem| stem.to_string())
// }

// /// 格式化文件大小
// pub fn format_file_size(size: u64) -> String {
//     const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
//     let mut size = size as f64;
//     let mut unit_index = 0;

//     while size >= 1024.0 && unit_index < UNITS.len() - 1 {
//         size /= 1024.0;
//         unit_index += 1;
//     }

//     if unit_index == 0 {
//         format!("{:.0} {}", size, UNITS[unit_index])
//     } else {
//         format!("{:.2} {}", size, UNITS[unit_index])
//     }
// }

/// 获取应用配置目录路径
pub fn get_config_dir() -> Result<PathBuf> {
    let appdata_dir = dirs::config_dir().ok_or_else(|| anyhow::anyhow!("无法获取 APPDATA 目录"))?;

    let biliup_dir = appdata_dir.join("biliup");

    // 确保目录存在
    if !biliup_dir.exists() {
        fs::create_dir_all(&biliup_dir)?;
    }

    Ok(biliup_dir)
}

/// 获取config.json文件路径
pub fn get_config_json_path() -> Result<PathBuf> {
    let config_dir = get_config_dir()?;
    Ok(config_dir.join("config.json"))
}

/// 获取config.yaml文件路径
pub fn get_config_yaml_path() -> Result<PathBuf> {
    let config_dir = get_config_dir()?;
    Ok(config_dir.join("config.yaml"))
}

// 获取cookie文件路径
pub fn get_old_cookie_file_path() -> Result<PathBuf> {
    let config_dir: PathBuf = get_config_dir()?;
    let users_dir = config_dir.join("users");
    if !users_dir.exists() {
        fs::create_dir_all(&users_dir)?;
    }

    // 获取第一个json文件
    let mut entries = fs::read_dir(&users_dir)?
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().and_then(|ext| ext.to_str()) == Some("json"))
        .map(|e| e.path());
    if let Some(first_file) = entries.next() {
        info!("找到cookie文件: {}", first_file.display());
        Ok(first_file)
    } else {
        Err(anyhow::anyhow!("没有找到cookie文件"))
    }
}
