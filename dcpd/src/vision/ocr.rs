//! Tesseract OCR integration.
//!
//! Provides text recognition from captured images using Tesseract.
//! Requires tesseract and leptonica system libraries.

use anyhow::Result;
use dcp_types::{Rect, TextBox, VisionOcrParams, VisionOcrResult};

/// Perform OCR on a base64-encoded image.
pub async fn ocr_image(params: &VisionOcrParams) -> Result<VisionOcrResult> {
    use base64::Engine;
    use image::GenericImageView;

    let image_data = base64::engine::general_purpose::STANDARD.decode(&params.image_base64)?;

    let img = image::load_from_memory(&image_data)
        .map_err(|e| anyhow::anyhow!("Failed to load image: {e}"))?;

    let (width, height) = img.dimensions();

    let cropped = if let Some(region) = &params.region {
        let x = region.x.max(0) as u32;
        let y = region.y.max(0) as u32;
        let w = region.width.min(width.saturating_sub(x));
        let h = region.height.min(height.saturating_sub(y));

        if w == 0 || h == 0 {
            return Ok(VisionOcrResult {
                text: String::new(),
                confidence: 0.0,
                text_boxes: vec![],
            });
        }

        let view = img.view(x, y, w, h).to_image();
        let rgb: image::ImageBuffer<image::Rgb<u8>, Vec<u8>> =
            image::ImageBuffer::from_fn(view.width(), view.height(), |px, py| {
                let pixel = view.get_pixel(px, py);
                image::Rgb([pixel[0], pixel[1], pixel[2]])
            });
        image::DynamicImage::ImageRgb8(rgb)
    } else {
        img.clone()
    };

    let temp_path = std::env::temp_dir().join(format!("dcp_ocr_{}.png", uuid::Uuid::new_v4()));
    cropped.save(&temp_path)?;

    let lang = params.language.as_deref().unwrap_or("eng");

    let mut tess = tesseract::Tesseract::new(Some(temp_path.to_str().unwrap_or("")), Some(lang))?;

    let text = tess.get_text()?;
    let hocr = tess.get_hocr_text(0).unwrap_or_default();
    let confidence = tess.mean_text_conf() as f64 / 100.0;

    let text_boxes = parse_hocr(&hocr);

    let _ = std::fs::remove_file(&temp_path);

    Ok(VisionOcrResult {
        text: text.trim().to_string(),
        confidence,
        text_boxes,
    })
}

fn parse_hocr(hocr: &str) -> Vec<TextBox> {
    let mut boxes = Vec::new();

    for line in hocr.lines() {
        if !line.contains("ocrx_word") {
            continue;
        }

        if let Some(bbox_start) = line.find("bbox") {
            let bbox_str = &line[bbox_start..];
            let numbers: Vec<i32> = bbox_str
                .split_whitespace()
                .skip(1)
                .take(4)
                .filter_map(|s| {
                    s.trim_end_matches(';')
                        .chars()
                        .take_while(|c| c.is_ascii_digit() || *c == '-')
                        .collect::<String>()
                        .parse()
                        .ok()
                })
                .collect();

            if numbers.len() == 4 {
                let x0 = numbers[0];
                let y0 = numbers[1];
                let x1 = numbers[2];
                let y1 = numbers[3];

                let confidence = if let Some(conf_start) = line.find("x_wconf") {
                    let conf_str = &line[conf_start..];
                    conf_str
                        .split_whitespace()
                        .nth(1)
                        .and_then(|s| {
                            s.chars()
                                .take_while(|c| c.is_ascii_digit() || *c == '.')
                                .collect::<String>()
                                .parse::<f64>()
                                .ok()
                        })
                        .unwrap_or(0.0)
                        / 100.0
                } else {
                    0.0
                };

                let text = if let Some(gt_pos) = line.find('>') {
                    let text_part = &line[gt_pos + 1..];
                    text_part.split('<').next().unwrap_or("").trim().to_string()
                } else {
                    String::new()
                };

                if !text.is_empty() {
                    boxes.push(TextBox {
                        bounds: Rect::new(x0, y0, (x1 - x0) as u32, (y1 - y0) as u32),
                        text,
                        confidence,
                    });
                }
            }
        }
    }

    boxes
}

pub fn is_available() -> bool {
    tesseract::Tesseract::new(Some(""), Some("eng")).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hocr_basic() {
        let hocr = r#"
<span class='ocrx_word' title='bbox 0 0 100 20; x_wconf 95'>Hello</span>
<span class='ocrx_word' title='bbox 100 0 200 20; x_wconf 90'>World</span>
"#;
        let boxes = parse_hocr(hocr);
        assert_eq!(boxes.len(), 2);
        assert_eq!(boxes[0].text, "Hello");
        assert_eq!(boxes[0].confidence, 0.95);
        assert_eq!(boxes[0].bounds, Rect::new(0, 0, 100, 20));
        assert_eq!(boxes[1].text, "World");
        assert_eq!(boxes[1].confidence, 0.90);
    }

    #[test]
    fn test_parse_hocr_empty() {
        let boxes = parse_hocr("");
        assert!(boxes.is_empty());
    }

    #[test]
    fn test_parse_hocr_no_ocrx_word() {
        let hocr = "<p>Some text without ocrx_word</p>";
        let boxes = parse_hocr(hocr);
        assert!(boxes.is_empty());
    }

    #[test]
    fn test_parse_hocr_missing_bbox() {
        let hocr = "<span class='ocrx_word'>No bbox here</span>";
        let boxes = parse_hocr(hocr);
        assert!(boxes.is_empty());
    }

    #[test]
    fn test_parse_hocr_partial_bbox() {
        let hocr = "<span class='ocrx_word' title='bbox 0 0 100'>Incomplete</span>";
        let boxes = parse_hocr(hocr);
        assert!(boxes.is_empty());
    }

    #[test]
    fn test_parse_hocr_without_confidence() {
        let hocr = "<span class='ocrx_word' title='bbox 0 0 50 20'>NoConf</span>";
        let boxes = parse_hocr(hocr);
        assert_eq!(boxes.len(), 1);
        assert_eq!(boxes[0].confidence, 0.0);
    }

    #[test]
    fn test_parse_hocr_multiple_lines() {
        let hocr = r#"
<span class='ocrx_word' title='bbox 0 0 30 15; x_wconf 85'>Line1</span>
<span class='ocrx_word' title='bbox 0 15 40 30; x_wconf 75'>Line2</span>
<span class='ocrx_word' title='bbox 0 30 35 45; x_wconf 88'>Line3</span>
"#;
        let boxes = parse_hocr(hocr);
        assert_eq!(boxes.len(), 3);
    }

    #[test]
    fn test_parse_hocr_handles_bad_confidence_value() {
        let hocr = "<span class='ocrx_word' title='bbox 0 0 50 20; x_wconf invalid'>Bad</span>";
        let boxes = parse_hocr(hocr);
        assert_eq!(boxes.len(), 1);
        assert_eq!(boxes[0].confidence, 0.0);
    }
}
