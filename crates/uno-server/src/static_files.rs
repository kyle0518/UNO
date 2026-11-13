use std::path::Path;

use tower_http::services::{ServeDir, ServeFile};

/// 建立 `/models` 靜態目錄服務（無 SPA fallback，找不到檔案直接回 404）。
pub fn models_service(models_dir: &Path) -> ServeDir {
    ServeDir::new(models_dir)
}

/// 建立前端 SPA 靜態檔案與 `index.html` fallback 服務。
pub fn spa_service(web_dir: &Path) -> ServeDir<ServeFile> {
    let index_path = web_dir.join("index.html");
    ServeDir::new(web_dir).fallback(ServeFile::new(index_path))
}
