//! Screen capture for Linux (X11 and Wayland).
//!
//! Uses `import` (ImageMagick) on X11 and `grim` on Wayland.

use anyhow::{Result, Context};
use dcp_types::{CaptureTarget, ImageFormat, VisionCaptureParams, VisionCaptureResult, Rect};
use base64::Engine;
use std::process::Command;
use tracing::warn;

/// Capture a screenshot on Linux.
pub fn capture_screen(params: &VisionCaptureParams) -> Result<VisionCaptureResult> {
    let is_wayland = std::env::var("WAYLAND_DISPLAY").is_ok();

    let (image_data, width, height, format) = match &params.target {
        CaptureTarget::Screen { monitor_id } => {
            if is_wayland {
                capture_grim(monitor_id.as_ref())?
            } else {
                capture_import_x11(monitor_id.as_ref())?
            }
        }
        CaptureTarget::Window { window_id } => {
            if is_wayland {
                // Wayland doesn't support per-window capture easily
                capture_grim(None)?
            } else {
                capture_window_x11(*window_id)?
            }
        }
        CaptureTarget::Region { bounds } => {
            if is_wayland {
                capture_grim_region(bounds)?
            } else {
                capture_import_x11_region(bounds)?
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

fn capture_grim(monitor_id: Option<&u64>) -> Result<(Vec<u8>, u32, u32, ImageFormat)> {
    // Use grim for Wayland screenshots
    let output = Command::new("grim")
        .args(["-t", "png", "-"])
        .output()
        .context("grim not found — install it for Wayland screenshots")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("grim failed: {stderr}");
    }

    // Parse PNG dimensions from header
    let (w, h) = parse_png_dimensions(&output.stdout).unwrap_or((0, 0));

    Ok((output.stdout, w, h, ImageFormat::Png))
}

fn capture_grim_region(bounds: &Rect) -> Result<(Vec<u8>, u32, u32, ImageFormat)> {
    let output = Command::new("grim")
        .args([
            "-t", "png",
            "-g", &format!("{}x{}+{}+{}", bounds.width, bounds.height, bounds.x, bounds.y),
            "-",
        ])
        .output()
        .context("grim not found")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("grim failed: {stderr}");
    }

    let (w, h) = parse_png_dimensions(&output.stdout).unwrap_or((0, 0));
    Ok((output.stdout, w, h, ImageFormat::Png))
}

fn capture_import_x11(monitor_id: Option<&u64>) -> Result<(Vec<u8>, u32, u32, ImageFormat)> {
    // Use import (ImageMagick) for X11 screenshots
    let root = if monitor_id.is_some() { "root" } else { "-window" };
    let args: Vec<&str> = if monitor_id.is_some() {
        vec!["-window", root, "png:-"]
    } else {
        vec!["png:-"]
    };

    let output = Command::new("import")
        .args(&args)
        .output()
        .context("import not found — install ImageMagick for X11 screenshots")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("import failed: {stderr}");
    }

    let (w, h) = parse_png_dimensions(&output.stdout).unwrap_or((0, 0));
    Ok((output.stdout, w, h, ImageFormat::Png))
}

fn capture_import_x11_region(bounds: &Rect) -> Result<(Vec<u8>, u32, u32, ImageFormat)> {
    let output = Command::new("import")
        .args([
            "-window", "root",
            "-crop", &format!("{}x{}+{}+{}", bounds.width, bounds.height, bounds.x, bounds.y),
            "png:-",
        ])
        .output()
        .context("import not found")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("import failed: {stderr}");
    }

    let (w, h) = parse_png_dimensions(&output.stdout).unwrap_or((0, 0));
    Ok((output.stdout, w, h, ImageFormat::Png))
}

fn capture_window_x11(window_id: u64) -> Result<(Vec<u8>, u32, u32, ImageFormat)> {
    let output = Command::new("import")
        .args(["-window", &window_id.to_string(), "png:-"])
        .output()
        .context("import not found")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("import failed: {stderr}");
    }

    let (w, h) = parse_png_dimensions(&output.stdout).unwrap_or((0, 0));
    Ok((output.stdout, w, h, ImageFormat::Png))
}

/// Parse PNG dimensions from the IHDR chunk.
fn parse_png_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    // PNG header: 8 bytes signature + IHDR chunk
    // IHDR chunk: 4 bytes length + 4 bytes "IHDR" + 4 bytes width + 4 bytes height
    if data.len() < 24 || &data[0..8] != b"\x89PNG\r\n\x1a\n" {
        return None;
    }
    let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
    let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
    Some((width, height))
}
