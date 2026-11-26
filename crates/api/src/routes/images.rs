//! Routes Module 4: Image Toolbox xử lý song song CPU bằng Rayon + tải về ảnh lẻ hoặc ZIP.

use axum::{
    Json,
    extract::{Multipart, State},
    http::{HeaderMap, HeaderValue, header},
    response::{IntoResponse, Response},
};
use pum_core::{AppError, Permission};
use pum_db::{models::ImageJobRow, resources};
use pum_modules::imagelab::{ImageProcessOptions, InputImageFile, OutputFormat, process_batch};

use crate::{error::ApiResult, extractors::AuthUser, state::AppState};

/// `GET /api/v1/images/jobs`
pub async fn list_image_jobs(
    State(state): State<AppState>,
    AuthUser(principal): AuthUser,
) -> ApiResult<Json<Vec<ImageJobRow>>> {
    let can_view_all = principal.has(Permission::AuditRead);
    let jobs = resources::list_image_jobs(&state.pool, principal.user_id, can_view_all).await?;
    Ok(Json(jobs))
}

/// `POST /api/v1/images/process` — Multipart upload (nhiều file) -> nén/resize/chuyển định dạng song song.
pub async fn process_images(
    State(state): State<AppState>,
    AuthUser(principal): AuthUser,
    mut multipart: Multipart,
) -> ApiResult<Response> {
    principal.require(Permission::ImagesProcess)?;

    let mut format = OutputFormat::Webp;
    let mut quality: u8 = 82;
    let mut max_width: Option<u32> = None;
    let mut max_height: Option<u32> = None;
    let mut grayscale = false;
    let mut watermark_strip = false;
    let mut files: Vec<InputImageFile> = Vec::new();
    let max_file_bytes = state.config.max_upload_bytes;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|err| AppError::Validation(format!("dữ liệu multipart không hợp lệ: {err}")))?
    {
        let field_name = field.name().unwrap_or_default().to_owned();
        if field_name == "files" || field_name == "file" {
            let filename = field
                .file_name()
                .map_or_else(|| format!("image_{}.png", files.len() + 1), str::to_owned);
            let data = field.bytes().await.map_err(|err| {
                AppError::Validation(format!("không thể đọc file tải lên: {err}"))
            })?;
            if data.len() > max_file_bytes {
                return Err(AppError::Validation(format!(
                    "file `{filename}` vượt quá giới hạn {} MB",
                    max_file_bytes / (1024 * 1024)
                ))
                .into());
            }
            if !data.is_empty() {
                files.push(InputImageFile {
                    filename,
                    bytes: data.to_vec(),
                });
            }
        } else {
            let text = field.text().await.unwrap_or_default();
            match field_name.as_str() {
                "format" => format = OutputFormat::parse(&text)?,
                "quality" => {
                    if let Ok(q) = text.trim().parse::<u8>() {
                        quality = q.clamp(10, 100);
                    }
                }
                "max_width" => {
                    max_width = text.trim().parse::<u32>().ok().filter(|&v| v > 0);
                }
                "max_height" => {
                    max_height = text.trim().parse::<u32>().ok().filter(|&v| v > 0);
                }
                "grayscale" => {
                    grayscale = matches!(
                        text.trim().to_ascii_lowercase().as_str(),
                        "true" | "1" | "on"
                    );
                }
                "watermark" => {
                    watermark_strip = matches!(
                        text.trim().to_ascii_lowercase().as_str(),
                        "true" | "1" | "on"
                    );
                }
                _ => {}
            }
        }
    }

    let opts = ImageProcessOptions {
        format,
        quality,
        max_width,
        max_height,
        grayscale,
        watermark_strip,
    };

    let result = tokio::task::spawn_blocking(move || process_batch(&files, &opts))
        .await
        .map_err(|err| AppError::Internal(format!("luồng xử lý ảnh gặp lỗi: {err}")))??;

    // Ghi nhận lịch sử job vào PostgreSQL
    let _ = resources::insert_image_job(
        &state.pool,
        principal.user_id,
        i32::try_from(result.file_count).unwrap_or(1),
        format.extension(),
        i32::from(quality),
        i64::try_from(result.original_bytes).unwrap_or(0),
        i64::try_from(result.processed_bytes).unwrap_or(0),
        i32::try_from(result.duration_ms).unwrap_or(0),
    )
    .await;

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(result.content_type),
    );
    if let Ok(disp) = HeaderValue::from_str(&format!(
        "attachment; filename=\"{}\"",
        result.download_filename
    )) {
        headers.insert(header::CONTENT_DISPOSITION, disp);
    }
    if let Ok(v) = HeaderValue::from_str(&result.original_bytes.to_string()) {
        headers.insert("x-pum-original-bytes", v);
    }
    if let Ok(v) = HeaderValue::from_str(&result.processed_bytes.to_string()) {
        headers.insert("x-pum-processed-bytes", v);
    }
    if let Ok(v) = HeaderValue::from_str(&result.duration_ms.to_string()) {
        headers.insert("x-pum-duration-ms", v);
    }
    headers.insert(
        header::ACCESS_CONTROL_EXPOSE_HEADERS,
        HeaderValue::from_static(
            "Content-Disposition, X-Pum-Original-Bytes, X-Pum-Processed-Bytes, X-Pum-Duration-Ms",
        ),
    );

    Ok((headers, result.payload).into_response())
}
