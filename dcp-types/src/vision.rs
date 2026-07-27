use crate::platform::Rect;
use serde::{Deserialize, Serialize};

/// Target for screen/window capture.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CaptureTarget {
    Screen { monitor_id: Option<u64> },
    Window { window_id: u64 },
    Region { bounds: Rect },
}

/// Image format for captured frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    Png,
    Jpeg,
    Bmp,
    Raw,
}

/// Parameters for `vision.capture`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisionCaptureParams {
    pub target: CaptureTarget,
    #[serde(default = "default_format")]
    pub format: ImageFormat,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_height: Option<u32>,
}

fn default_format() -> ImageFormat {
    ImageFormat::Png
}

/// Result of a capture operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisionCaptureResult {
    pub width: u32,
    pub height: u32,
    pub format: ImageFormat,
    pub data_base64: String,
    pub timestamp: i64,
}

/// Parameters for `vision.ocr`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisionOcrParams {
    pub image_base64: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<Rect>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

/// Result of OCR.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisionOcrResult {
    pub text: String,
    pub confidence: f64,
    pub text_boxes: Vec<TextBox>,
}

/// A recognized text region with bounding box.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextBox {
    pub bounds: Rect,
    pub text: String,
    pub confidence: f64,
}

/// UI element detection result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedElement {
    pub bounds: Rect,
    pub element_type: UiElementType,
    pub label: Option<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UiElementType {
    Button,
    TextField,
    Label,
    Icon,
    Checkbox,
    Dropdown,
    Scrollbar,
    Tab,
    Link,
    Dialog,
    ErrorDialog,
    Terminal,
    CodeBlock,
    Image,
    Menu,
    MenuItem,
    Slider,
    Unknown,
}

/// Parameters for `vision.detect_elements`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisionDetectParams {
    pub image_base64: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<Rect>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element_types: Option<Vec<UiElementType>>,
}

/// Result of UI element detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisionDetectResult {
    pub elements: Vec<DetectedElement>,
}
