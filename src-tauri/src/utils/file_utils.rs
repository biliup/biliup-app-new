use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tracing::{error, info};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
}

/// 获取文件大小
pub fn get_file_size(path: &Path) -> Result<u64> {
    let metadata = std::fs::metadata(path)?;
    Ok(metadata.len())
}

/// 递归读取目录，返回所有文件（可选择是否包含子目录）
pub fn read_dir_recursive(
    dir_path: &Path,
    include_subdirs: bool,
    max_depth: Option<u32>,
) -> Result<Vec<FileEntry>> {
    let mut result = Vec::new();
    read_dir_recursive_internal(
        dir_path,
        include_subdirs,
        max_depth.unwrap_or(20),
        0,
        &mut result,
    )?;
    Ok(result)
}

fn read_dir_recursive_internal(
    dir_path: &Path,
    include_subdirs: bool,
    max_depth: u32,
    current_depth: u32,
    result: &mut Vec<FileEntry>,
) -> Result<()> {
    // 防止递归深度过深
    if current_depth > max_depth {
        return Ok(());
    }

    // 检查目录是否存在和可读
    if !dir_path.exists() || !dir_path.is_dir() {
        return Err(anyhow::anyhow!(
            "Directory does not exist or is not readable: {:?}",
            dir_path
        ));
    }

    let entries = match fs::read_dir(dir_path) {
        Ok(entries) => entries,
        Err(e) => {
            // 记录警告但继续处理其他目录
            error!("Warning: Failed to read directory {:?}: {}", dir_path, e);
            return Ok(());
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                error!("Warning: Failed to read directory entry: {}", e);
                continue;
            }
        };

        let path = entry.path();
        let file_name = match entry.file_name().to_str() {
            Some(name) => name.to_string(),
            None => continue, // 跳过无法转换为字符串的文件名
        };

        // 跳过隐藏文件和系统文件夹
        if file_name.starts_with('.')
            || file_name.to_lowercase().contains("system")
            || file_name.to_lowercase().contains("$recycle")
            || file_name.to_lowercase().contains("windows")
        {
            continue;
        }

        let path_str = match path.to_str() {
            Some(path_str) => path_str.to_string(),
            None => continue,
        };

        if path.is_dir() {
            if include_subdirs {
                // 递归处理子目录
                if let Err(e) = read_dir_recursive_internal(
                    &path,
                    include_subdirs,
                    max_depth,
                    current_depth + 1,
                    result,
                ) {
                    error!("Warning: Failed to read subdirectory {:?}: {}", path, e);
                }
            }
        } else {
            // 添加文件到结果中
            result.push(FileEntry {
                name: file_name,
                path: path_str,
                is_directory: false,
            });
        }
    }

    Ok(())
}

/// 获取日志路径
pub fn get_log_path() -> Result<PathBuf> {
    let config_dir = get_config_dir()?;
    let log_dir = config_dir.join("logs");

    // 确保日志目录存在
    if !log_dir.exists() {
        fs::create_dir_all(&log_dir)?;
    }

    Ok(log_dir)
}

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
