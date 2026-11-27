use std::sync::Arc;

use tauri::AppHandle;
use telai_stream::{Asset, AssetSource};

/// Serves the player from the frontend files embedded in the app.
#[cfg(not(debug_assertions))]
pub fn source(app: AppHandle) -> AssetSource {
    Arc::new(move |path| {
        let asset = app.asset_resolver().get(path.to_owned())?;

        Some(Asset {
            body: asset.bytes,
            mime_type: asset.mime_type,
        })
    })
}

/// Development builds load the frontend from the Vite dev server and embed
/// nothing, so the player comes from the last `pnpm build` output.
#[cfg(debug_assertions)]
pub fn source(_app: AppHandle) -> AssetSource {
    let dist = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../dist");

    Arc::new(move |path| {
        let body = std::fs::read(dist.join(path)).ok()?;

        Some(Asset {
            body,
            mime_type: mime_type(path).to_owned(),
        })
    })
}

#[cfg(debug_assertions)]
fn mime_type(path: &str) -> &'static str {
    match path.rsplit_once('.').map(|(_, extension)| extension) {
        Some("html") => "text/html",
        Some("js") => "text/javascript",
        Some("css") => "text/css",
        Some("svg") => "image/svg+xml",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}
