use std::path::PathBuf;

use iced_wgpu::wgpu;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::cache::HexDigest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Format {
    #[serde(with = "serde_image_format")]
    Raster(image::ImageFormat),
    Svg,
}

impl Format {
    pub fn from_magic_bytes(bytes: &[u8]) -> Option<Format> {
        image::guess_format(bytes).ok().map(Format::Raster)
    }

    pub fn from_mime_type(mime_type: &str) -> Option<Format> {
        match mime_type {
            "image/svg+xml" | "image/svg+xml; charset=utf-8" => {
                Some(Format::Svg)
            }
            _ => image::ImageFormat::from_mime_type(mime_type)
                .map(Format::Raster),
        }
    }

    pub fn to_mime_type(&self) -> &'static str {
        match self {
            Format::Raster(format) => format.to_mime_type(),
            Format::Svg => "image/svg+xml",
        }
    }

    pub fn extensions_str(&self) -> &'static [&'static str] {
        match self {
            Format::Raster(format) => format.extensions_str(),
            Format::Svg => &["svg"],
        }
    }
}

pub type Error = image::ImageError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    pub format: Format,
    pub url: Url,
    pub digest: HexDigest,
    pub path: PathBuf,
}

impl Image {
    pub fn new(
        format: Format,
        url: Url,
        digest: HexDigest,
        path: PathBuf,
    ) -> Self {
        Self {
            format,
            url,
            digest,
            path,
        }
    }
}

/// Whether an image of these pixel dimensions would exceed the maximum GPU
/// buffer size once its rows are padded for upload. Shared decompression-bomb
/// guard for the link-preview and server-icon fetchers; returns the computed
/// `(padded_size, max_buffer_size)` when it overflows so callers can surface the
/// sizes.
pub fn gpu_buffer_overflow(width: u32, height: u32) -> Option<(u64, u64)> {
    // As per iced, webgpu requires
    //   BufferCopyView.layout.bytes_per_row % COPY_BYTES_PER_ROW_ALIGNMENT == 0
    // so round the row width up to the next multiple first.
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padding = (align - (4 * width) % align) % align;
    let padded_width = u64::from(4 * width + padding);
    let padded_size = padded_width * u64::from(height);
    let max_buffer_size = wgpu::Limits::downlevel_defaults().max_buffer_size;

    (padded_size > max_buffer_size).then_some((padded_size, max_buffer_size))
}

mod serde_image_format {
    use image::ImageFormat;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(
        format: &ImageFormat,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        format.to_mime_type().serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<ImageFormat, D::Error> {
        let s = String::deserialize(deserializer)?;

        ImageFormat::from_mime_type(s)
            .ok_or(serde::de::Error::custom("invalid mime type"))
    }
}
