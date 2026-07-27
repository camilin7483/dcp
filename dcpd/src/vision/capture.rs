//! Screen capture for Linux (X11 and Wayland).
//!
//! Uses `import` (ImageMagick) on X11 and `grim` on Wayland.

use anyhow::{Result, Context};
use dcp_types::{CaptureTarget, ImageFormat, VisionCaptureParams, VisionCaptureResult, Rect};
use base64::Engine;
use tracing::warn;

/// Capture a screenshot on Linux.
pub async fn capture_screen(params: &VisionCaptureParams) -> Result<VisionCaptureResult> {
    let is_wayland = std::env::var("WAYLAND_DISPLAY").is_ok();

    let (image_data, width, height, _format) = match &params.target {
        CaptureTarget::Screen { monitor_id } => {
            if is_wayland {
                capture_grim(monitor_id.as_ref()).await?
            } else {
                capture_import_x11(monitor_id.as_ref()).await?
            }
        }
        CaptureTarget::Window { window_id } => {
            if is_wayland {
                capture_grim(None).await?
            } else {
                capture_window_x11(*window_id).await?
            }
        }
        CaptureTarget::Region { bounds } => {
            if is_wayland {
                capture_grim_region(bounds).await?
            } else {
                capture_import_x11_region(bounds).await?
            }
        }
    };

    let format = match params.format {
        ImageFormat::Png => ImageFormat::Png,
        ImageFormat::Jpeg => ImageFormat::Jpeg,
        ImageFormat::Bmp => ImageFormat::Bmp,
        ImageFormat::Raw => ImageFormat::Raw,
    };

    Ok(VisionCaptureResult {
        width,
        height,
        format,
        data_base64: base64::engine::general_purpose::STANDARD.encode(&image_data),
        timestamp: chrono::Utc::now().timestamp_millis(),
    })
}

async fn capture_grim(_monitor_id: Option<&u64>) -> Result<(Vec<u8>, u32, u32, ImageFormat)> {
    let output = tokio::process::Command::new("grim")
        .args(["-t", "png", "-"])
        .output().await
        .context("grim not found — install it for Wayland screenshots")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("grim failed: {stderr}");
    }

    let (w, h) = parse_png_dimensions(&output.stdout).unwrap_or((0, 0));
    if w == 0 || h == 0 {
        warn!("Could not parse PNG dimensions from grim output");
    }

    Ok((output.stdout, w, h, ImageFormat::Png))
}

async fn capture_grim_region(bounds: &Rect) -> Result<(Vec<u8>, u32, u32, ImageFormat)> {
    let output = tokio::process::Command::new("grim")
        .args([
            "-t", "png",
            "-g", &format!("{}x{}+{}+{}", bounds.width, bounds.height, bounds.x, bounds.y),
            "-",
        ])
        .output().await
        .context("grim not found")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("grim failed: {stderr}");
    }

    let (w, h) = parse_png_dimensions(&output.stdout).unwrap_or((0, 0));
    if w == 0 || h == 0 {
        warn!("Could not parse PNG dimensions from grim region capture output");
    }

    Ok((output.stdout, w, h, ImageFormat::Png))
}

async fn capture_import_x11(monitor_id: Option<&u64>) -> Result<(Vec<u8>, u32, u32, ImageFormat)> {
    let mut args = vec!["-window", "root"];
    if let Some(_id) = monitor_id {
        // Note: xrandr-based monitor selection is complex
        // For now, just capture full screen
    }
    args.push("PNG:-");

    let output = tokio::process::Command::new("import")
        .args(&args)
        .output().await
        .context("import not found — install ImageMagick for X11 screenshots")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("import failed: {stderr}");
    }

    let (w, h) = parse_png_dimensions(&output.stdout).unwrap_or((0, 0));
    if w == 0 || h == 0 {
        warn!("Could not parse PNG dimensions from import output");
    }

    Ok((output.stdout, w, h, ImageFormat::Png))
}

async fn capture_import_x11_region(bounds: &Rect) -> Result<(Vec<u8>, u32, u32, ImageFormat)> {
    let output = tokio::process::Command::new("import")
        .args([
            "-window", "root",
            "-crop", &format!("{}x{}+{}+{}", bounds.width, bounds.height, bounds.x, bounds.y),
            "PNG:-",
        ])
        .output().await
        .context("import not found")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("import failed: {stderr}");
    }

    let (w, h) = parse_png_dimensions(&output.stdout).unwrap_or((0, 0));
    if w == 0 || h == 0 {
        warn!("Could not parse PNG dimensions from import region capture output");
    }

    Ok((output.stdout, w, h, ImageFormat::Png))
}

async fn capture_window_x11(window_id: u64) -> Result<(Vec<u8>, u32, u32, ImageFormat)> {
    let output = tokio::process::Command::new("import")
        .args(["-window", &window_id.to_string(), "PNG:-"])
        .output().await
        .context("import not found")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("import failed: {stderr}");
    }

    let (w, h) = parse_png_dimensions(&output.stdout).unwrap_or((0, 0));
    if w == 0 || h == 0 {
        warn!("Could not parse PNG dimensions from import window capture output");
    }

    Ok((output.stdout, w, h, ImageFormat::Png))
}

/// Parse PNG dimensions from the IHDR chunk.
fn parse_png_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    // PNG header: 8 bytes signature + IHDR chunk
    // IHDR chunk: 4 bytes length + 4 bytes "IHDR" + 4 bytes width + 4 bytes height
    if data.len() < 24 || &data[0..8] != b"\x89PNG\r\n\x1a\n" {
        warn!("Data too short or invalid PNG signature for dimension parsing");
        return None;
    }
    let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
    let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
    Some((width, height))
}
