use std::env;
use std::io::Cursor;

use image::imageops::FilterType;
use image::ImageFormat;
use tauri::{Runtime, WebviewWindow};

// Platform-specific modules
#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "ios")]
mod ios;

#[cfg(target_os = "android")]
mod android;

/// Environment variable name for default max width
const ENV_MAX_WIDTH: &str = "TAURI_MCP_SCREENSHOT_MAX_WIDTH";

/// Screenshot result containing the image data
#[derive(Debug)]
pub struct Screenshot {
    /// The raw PNG bytes
    pub data: Vec<u8>,
}

/// Screenshot error types
#[derive(Debug, thiserror::Error)]
pub enum ScreenshotError {
    #[error("Platform not supported")]
    PlatformUnsupported,

    #[error("Webview capture failed: {0}")]
    CaptureFailed(String),

    #[error("Encoding failed: {0}")]
    EncodeFailed(String),

    #[error("Resize failed: {0}")]
    ResizeFailed(String),

    #[error("Timeout exceeded")]
    Timeout,
}

/// Get the effective max_width value.
/// Priority: param > env var > None
fn get_effective_max_width(param: Option<u32>) -> Option<u32> {
    if param.is_some() {
        return param;
    }

    env::var(ENV_MAX_WIDTH)
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
}

/// Convert PNG data to JPEG format with specified quality.
fn convert_to_jpeg(png_data: Vec<u8>, quality: u8) -> Result<Vec<u8>, ScreenshotError> {
    let img = image::load_from_memory(&png_data)
        .map_err(|e| ScreenshotError::EncodeFailed(format!("Failed to decode PNG: {e}")))?;

    let mut buffer = Cursor::new(Vec::new());
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, quality);

    img.write_with_encoder(encoder)
        .map_err(|e| ScreenshotError::EncodeFailed(format!("Failed to encode JPEG: {e}")))?;

    Ok(buffer.into_inner())
}

/// Resize image data if it exceeds max_width, preserving aspect ratio.
/// Returns the original data if no resizing is needed.
/// Note: This function only handles resizing, not format conversion.
fn resize_if_needed(
    data: Vec<u8>,
    max_width: u32,
) -> Result<Vec<u8>, ScreenshotError> {
    let img = image::load_from_memory(&data)
        .map_err(|e| ScreenshotError::ResizeFailed(format!("Failed to decode image: {e}")))?;

    let current_width = img.width();

    // Only resize if image is wider than max_width (never upscale)
    if current_width <= max_width {
        return Ok(data);
    }

    // Calculate new dimensions preserving aspect ratio
    let current_height = img.height();
    let scale = max_width as f64 / current_width as f64;
    let new_height = (current_height as f64 * scale).round() as u32;

    // Resize using Lanczos3 for high quality
    let resized = img.resize(max_width, new_height, FilterType::Lanczos3);

    // Encode back to PNG (format conversion happens later)
    let mut buffer = Cursor::new(Vec::new());
    resized
        .write_to(&mut buffer, ImageFormat::Png)
        .map_err(|e| ScreenshotError::ResizeFailed(format!("Failed to encode PNG: {e}")))?;

    Ok(buffer.into_inner())
}

/// Convert image data to the requested format (PNG or JPEG).
/// If data is already in the requested format, returns it unchanged.
fn convert_format(
    data: Vec<u8>,
    format: &str,
    quality: u8,
) -> Result<Vec<u8>, ScreenshotError> {
    // If PNG is requested, return as-is (platform implementations return PNG)
    if format == "png" {
        return Ok(data);
    }

    // Convert to JPEG
    convert_to_jpeg(data, quality)
}

/// Platform-specific screenshot implementation trait
pub trait PlatformScreenshot {
    /// Capture a screenshot of the current viewport
    fn capture_viewport(
        window: &WebviewWindow<impl Runtime>,
    ) -> Result<Screenshot, ScreenshotError>;
}

/// Capture a screenshot of the current viewport using platform-specific APIs
pub async fn capture_viewport_screenshot<R: Runtime>(
    window: &WebviewWindow<R>,
    format: &str,
    quality: u8,
    max_width: Option<u32>,
) -> Result<String, ScreenshotError> {
    // Dispatch to platform-specific implementation
    #[cfg(target_os = "macos")]
    let screenshot = macos::capture_viewport(window)?;

    #[cfg(target_os = "windows")]
    let screenshot = windows::capture_viewport(window)?;

    #[cfg(target_os = "linux")]
    let screenshot = linux::capture_viewport(window)?;

    #[cfg(target_os = "ios")]
    let screenshot = ios::capture_viewport(window)?;

    #[cfg(target_os = "android")]
    let screenshot = android::capture_viewport(window)?;

    #[cfg(not(any(
        target_os = "macos",
        target_os = "windows",
        target_os = "linux",
        target_os = "ios",
        target_os = "android"
    )))]
    return Err(ScreenshotError::PlatformUnsupported);

    // Apply max_width constraint if specified (param or env var)
    let effective_max_width = get_effective_max_width(max_width);
    let resized_data = match effective_max_width {
        Some(max_w) => resize_if_needed(screenshot.data, max_w)?,
        None => screenshot.data,
    };

    // Convert to the requested format (PNG data from platform -> JPEG if needed)
    let final_data = convert_format(resized_data, format, quality)?;

    // Convert to base64 data URL
    let mime_type = if format == "jpeg" {
        "image/jpeg"
    } else {
        "image/png"
    };

    use base64::Engine as _;
    let base64_data = base64::engine::general_purpose::STANDARD.encode(&final_data);
    let data_url = format!("data:{mime_type};base64,{base64_data}");

    Ok(data_url)
}
