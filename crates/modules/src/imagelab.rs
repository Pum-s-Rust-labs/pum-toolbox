//! Module 4: Hộp công cụ xử lý & nén ảnh song song đa nhân CPU bằng `image` + `rayon` + `zip`.
//! Kèm bảo vệ chống decompression bomb và kiểm tra magic bytes định dạng ảnh.

use std::io::{Cursor, Write};
use std::time::Instant;

use image::{DynamicImage, GenericImageView, ImageFormat, Rgba, codecs::jpeg::JpegEncoder};
use pum_core::{AppError, AppResult};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

/// Giới hạn an toàn chống decompression bomb (tối đa 10.000px mỗi cạnh và 40 triệu pixel).
const MAX_DIMENSION: u32 = 10_000;
const MAX_TOTAL_PIXELS: u64 = 40_000_000;
const MAX_BATCH_FILES: usize = 10;

/// Định dạng ảnh đầu ra được hỗ trợ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Webp,
    Jpeg,
    Png,
}

impl OutputFormat {
    /// Phân tích từ chuỗi form field.
    pub fn parse(raw: &str) -> AppResult<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "webp" => Ok(Self::Webp),
            "jpg" | "jpeg" => Ok(Self::Jpeg),
            "png" => Ok(Self::Png),
            other => Err(AppError::Validation(format!(
                "định dạng đầu ra `{other}` không hỗ trợ (dùng webp, jpeg, hoặc png)"
            ))),
        }
    }

    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Webp => "webp",
            Self::Jpeg => "jpg",
            Self::Png => "png",
        }
    }

    #[must_use]
    pub const fn content_type(self) -> &'static str {
        match self {
            Self::Webp => "image/webp",
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
        }
    }
}

/// Tham số xử lý cho một batch ảnh.
#[derive(Debug, Clone)]
pub struct ImageProcessOptions {
    pub format: OutputFormat,
    pub quality: u8,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    pub grayscale: bool,
    pub watermark_strip: bool,
}

/// Một file ảnh đầu vào.
#[derive(Debug, Clone)]
pub struct InputImageFile {
    pub filename: String,
    pub bytes: Vec<u8>,
}

/// Kết quả trả về sau khi xử lý toàn bộ batch.
#[derive(Debug)]
pub struct BatchProcessResult {
    pub payload: Vec<u8>,
    pub content_type: &'static str,
    pub download_filename: String,
    pub file_count: usize,
    pub original_bytes: usize,
    pub processed_bytes: usize,
    pub duration_ms: u128,
}

/// Xử lý một ảnh đơn lẻ: kiểm tra magic bytes, chống bomb, resize, grayscale, watermark, nén.
fn process_single_image(
    input: &InputImageFile,
    opts: &ImageProcessOptions,
) -> AppResult<(String, Vec<u8>)> {
    let detected = image::guess_format(&input.bytes).map_err(|_| {
        AppError::SecurityViolation(format!(
            "file `{}` không có magic bytes của định dạng ảnh hợp lệ",
            input.filename
        ))
    })?;

    if !matches!(
        detected,
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::WebP | ImageFormat::Gif
    ) {
        return Err(AppError::SecurityViolation(format!(
            "định dạng ảnh `{detected:?}` của file `{}` không được phép",
            input.filename
        )));
    }

    let reader = image::ImageReader::with_format(Cursor::new(&input.bytes), detected);
    let (width, height) = reader
        .into_dimensions()
        .map_err(|err| AppError::Validation(format!("không thể đọc kích thước ảnh: {err}")))?;

    if width > MAX_DIMENSION
        || height > MAX_DIMENSION
        || u64::from(width).saturating_mul(u64::from(height)) > MAX_TOTAL_PIXELS
    {
        return Err(AppError::SecurityViolation(format!(
            "ảnh `{}` có kích thước {width}x{height} vượt ngưỡng bảo vệ decompression bomb",
            input.filename
        )));
    }

    let mut img = image::load_from_memory_with_format(&input.bytes, detected)
        .map_err(|err| AppError::Validation(format!("giải mã ảnh thất bại: {err}")))?;

    if opts.max_width.is_some() || opts.max_height.is_some() {
        let target_w = opts.max_width.unwrap_or(u32::MAX).clamp(16, MAX_DIMENSION);
        let target_h = opts.max_height.unwrap_or(u32::MAX).clamp(16, MAX_DIMENSION);
        let (cur_w, cur_h) = img.dimensions();
        if cur_w > target_w || cur_h > target_h {
            img = img.resize(target_w, target_h, image::imageops::FilterType::Lanczos3);
        }
    }

    if opts.grayscale {
        img = DynamicImage::ImageLuma8(img.to_luma8());
    }

    if opts.watermark_strip {
        img = apply_corner_watermark(&img);
    }

    let encoded = encode_image(&img, opts.format, opts.quality)?;
    let stem = input
        .filename
        .rsplit_once('.')
        .map_or(input.filename.as_str(), |(s, _)| s);
    let safe_stem: String = stem
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(64)
        .collect();
    let final_name = format!(
        "{}.{}",
        if safe_stem.is_empty() {
            "image"
        } else {
            &safe_stem
        },
        opts.format.extension()
    );

    Ok((final_name, encoded))
}

/// Đóng dấu dải watermark mờ thanh lịch ở góc dưới bên phải ảnh (thuần Rust, không cần font hệ thống).
fn apply_corner_watermark(img: &DynamicImage) -> DynamicImage {
    let mut rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let strip_h = (h / 18).clamp(12, 48);
    let strip_w = (w / 4).clamp(48, 220);
    let start_x = w.saturating_sub(strip_w + 12);
    let start_y = h.saturating_sub(strip_h + 12);

    for y in start_y..h.saturating_sub(12) {
        for x in start_x..w.saturating_sub(12) {
            let p = rgba.get_pixel(x, y);
            let is_border = y == start_y
                || y + 1 == h.saturating_sub(12)
                || x == start_x
                || x + 1 == w.saturating_sub(12);
            let blended = if is_border {
                Rgba([56, 189, 248, 220])
            } else {
                Rgba([
                    u8::try_from((u16::from(p[0]) * 55 + 15 * 45) / 100).unwrap_or(15),
                    u8::try_from((u16::from(p[1]) * 55 + 23 * 45) / 100).unwrap_or(23),
                    u8::try_from((u16::from(p[2]) * 55 + 42 * 45) / 100).unwrap_or(42),
                    p[3],
                ])
            };
            rgba.put_pixel(x, y, blended);
        }
    }

    DynamicImage::ImageRgba8(rgba)
}

fn encode_image(img: &DynamicImage, format: OutputFormat, quality: u8) -> AppResult<Vec<u8>> {
    let mut buf = Cursor::new(Vec::new());
    match format {
        OutputFormat::Jpeg => {
            let rgb = img.to_rgb8();
            let mut encoder = JpegEncoder::new_with_quality(&mut buf, quality.clamp(10, 100));
            encoder
                .encode_image(&rgb)
                .map_err(|err| AppError::Internal(format!("nén JPEG thất bại: {err}")))?;
        }
        OutputFormat::Png => {
            img.write_to(&mut buf, ImageFormat::Png)
                .map_err(|err| AppError::Internal(format!("xuất PNG thất bại: {err}")))?;
        }
        OutputFormat::Webp => {
            img.write_to(&mut buf, ImageFormat::WebP)
                .map_err(|err| AppError::Internal(format!("xuất WebP thất bại: {err}")))?;
        }
    }
    Ok(buf.into_inner())
}

/// Xử lý song song cả batch ảnh bằng Rayon; trả trực tiếp ảnh nếu 1 file, hoặc đóng gói `.zip` nếu nhiều file.
pub fn process_batch(
    files: &[InputImageFile],
    opts: &ImageProcessOptions,
) -> AppResult<BatchProcessResult> {
    if files.is_empty() {
        return Err(AppError::Validation(
            "vui lòng tải lên ít nhất 1 file ảnh".to_owned(),
        ));
    }
    if files.len() > MAX_BATCH_FILES {
        return Err(AppError::Validation(format!(
            "tối đa {MAX_BATCH_FILES} ảnh mỗi lần xử lý"
        )));
    }

    let start = Instant::now();
    let original_bytes: usize = files.iter().map(|f| f.bytes.len()).sum();

    let results: AppResult<Vec<(String, Vec<u8>)>> = files
        .par_iter()
        .map(|file| process_single_image(file, opts))
        .collect();
    let mut processed = results?;

    let duration_ms = start.elapsed().as_millis();

    if processed.len() == 1 {
        let (filename, payload) = processed.pop().expect("checked len == 1");
        let processed_bytes = payload.len();
        return Ok(BatchProcessResult {
            payload,
            content_type: opts.format.content_type(),
            download_filename: filename,
            file_count: 1,
            original_bytes,
            processed_bytes,
            duration_ms,
        });
    }

    // Đóng gói ZIP khi có >= 2 file
    let mut zip_buf = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut zip_buf);
        let zip_opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (idx, (name, bytes)) in processed.iter().enumerate() {
            let unique_name = format!("{:02}_{name}", idx + 1);
            zip.start_file(unique_name, zip_opts)
                .map_err(|err| AppError::Internal(format!("lỗi tạo entry ZIP: {err}")))?;
            zip.write_all(bytes)
                .map_err(|err| AppError::Internal(format!("lỗi ghi dữ liệu ZIP: {err}")))?;
        }
        zip.finish()
            .map_err(|err| AppError::Internal(format!("lỗi đóng gói ZIP: {err}")))?;
    }

    let payload = zip_buf.into_inner();
    let processed_bytes = payload.len();

    Ok(BatchProcessResult {
        payload,
        content_type: "application/zip",
        download_filename: "pum-imagelab-batch.zip".to_owned(),
        file_count: files.len(),
        original_bytes,
        processed_bytes,
        duration_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};

    fn make_sample_png(width: u32, height: u32) -> Vec<u8> {
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(width, height, |x, y| {
            Rgb([
                u8::try_from(x % 256).unwrap_or(0),
                u8::try_from(y % 256).unwrap_or(0),
                128,
            ])
        });
        let mut buf = Cursor::new(Vec::new());
        DynamicImage::ImageRgb8(img)
            .write_to(&mut buf, ImageFormat::Png)
            .expect("png sample");
        buf.into_inner()
    }

    #[test]
    fn resizes_converts_and_zips_batch_in_parallel() {
        let sample = make_sample_png(120, 80);
        let opts = ImageProcessOptions {
            format: OutputFormat::Jpeg,
            quality: 80,
            max_width: Some(60),
            max_height: Some(60),
            grayscale: false,
            watermark_strip: true,
        };

        let single = process_batch(
            &[InputImageFile {
                filename: "photo.png".to_owned(),
                bytes: sample.clone(),
            }],
            &opts,
        )
        .expect("single image should process");
        assert_eq!(single.content_type, "image/jpeg");
        assert_eq!(single.download_filename, "photo.jpg");

        let batch = process_batch(
            &[
                InputImageFile {
                    filename: "a.png".to_owned(),
                    bytes: sample.clone(),
                },
                InputImageFile {
                    filename: "b.png".to_owned(),
                    bytes: sample,
                },
            ],
            &opts,
        )
        .expect("batch zip should process");
        assert_eq!(batch.content_type, "application/zip");
        assert_eq!(batch.file_count, 2);
    }

    #[test]
    fn rejects_non_image_payload() {
        let opts = ImageProcessOptions {
            format: OutputFormat::Webp,
            quality: 80,
            max_width: None,
            max_height: None,
            grayscale: false,
            watermark_strip: false,
        };
        let err = process_batch(
            &[InputImageFile {
                filename: "evil.png".to_owned(),
                bytes: b"<script>alert(1)</script>".to_vec(),
            }],
            &opts,
        )
        .expect_err("non-image must be rejected");
        assert!(matches!(err, AppError::SecurityViolation(_)));
    }
}
