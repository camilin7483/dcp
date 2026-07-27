//! Vision module: screen capture, OCR, element detection (optional).
//!
//! Screen capture is implemented using platform-specific tools:
//! - X11: `import` (ImageMagick) or `xwd`
//! - Wayland: `grim` or `slurp`

pub mod capture;
pub mod ocr;
pub mod detection;
