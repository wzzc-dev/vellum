use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::SystemTime;

use anyhow::Result;

/// 构建结果
#[derive(Debug, Clone)]
pub enum BuildResult {
    Success {
        wasm_path: PathBuf,
        duration_ms: u64,
    },
    Failure {
        error_message: String,
        duration_ms: u64,
    },
}

/// 构建缓存项
#[allow(dead_code)]
struct BuildCacheEntry {
    last_build_time: SystemTime,
    last_build_result: Option<BuildResult>,
    source_files: Vec<PathBuf>,
}

/// 构建控制器
pub struct BuildController {
    cache: std::collections::HashMap<String, BuildCacheEntry>,
}

impl BuildController {
    pub fn new() -> Self {
        Self {
            cache: std::collections::HashMap::new(),
        }
    }

    /// 构建扩展
    pub async fn build_extension(&mut self, extension_id: &str, directory: &Path) -> Result<BuildResult> {
        let start = SystemTime::now();
        
        let build_script = directory.join("build.sh");
        if !build_script.exists() {
            return Ok(BuildResult::Failure {
                error_message: format!("Build script not found: {}", build_script.display()),
                duration_ms: duration_ms(start),
            });
        }

        let build_output = Command::new("sh")
            .arg("-c")
            .arg(format!("cd '{}' && ./build.sh", directory.to_string_lossy()))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        let duration = duration_ms(start);

        if build_output.status.success() {
            let component_path = find_component_path(directory)?;
            Ok(BuildResult::Success {
                wasm_path: component_path,
                duration_ms: duration,
            })
        } else {
            let stderr = String::from_utf8_lossy(&build_output.stderr);
            Ok(BuildResult::Failure {
                error_message: stderr.to_string(),
                duration_ms: duration,
            })
        }
    }
}

fn duration_ms(start: SystemTime) -> u64 {
    start.elapsed().map(|d| d.as_millis() as u64).unwrap_or(0)
}

fn find_component_path(directory: &Path) -> Result<PathBuf> {
    let target_dir = directory.join("target/wasm32-wasip2/release");
    if target_dir.exists() {
        for entry in std::fs::read_dir(target_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "wasm").unwrap_or(false) {
                return Ok(path);
            }
        }
    }
    anyhow::bail!("Compiled component not found")
}

impl Default for BuildController {
    fn default() -> Self {
        Self::new()
    }
}
