use iced::widget;
use image::GenericImageView;
use url::Url;

#[derive(Debug, Clone)]
pub enum Format {
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

#[derive(Debug, Clone)]
pub enum ImageHandle {
    Raster(widget::image::Handle),
    Svg(widget::svg::Handle),
}

impl ImageHandle {
    pub fn to_vec(&self) -> Option<Vec<u8>> {
        match self {
            ImageHandle::Raster(handle) => match handle {
                widget::image::Handle::Bytes(_, bytes) => Some(bytes.to_vec()),
                _ => None,
            },
            ImageHandle::Svg(handle) => match handle.data() {
                iced_core::svg::Data::Bytes(cow) => Some(cow.to_vec()),
                _ => None,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct Image {
    pub format: Format,
    pub url: Url,
    pub handle: ImageHandle,
}

impl Image {
    pub fn new(format: Format, url: Url, data: Vec<u8>) -> Self {
        let handle = match format {
            Format::Raster(_) => {
                ImageHandle::Raster(widget::image::Handle::from_bytes(data))
            }
            Format::Svg => {
                ImageHandle::Svg(widget::svg::Handle::from_memory(data))
            }
        };

        Self {
            format,
            url,
            handle,
        }
    }

    pub fn suggested_file_name(&self) -> String {
        let ext = self.format.extensions_str()[0];

        let stem = self
            .url
            .path_segments()
            .and_then(|mut segments| segments.next_back())
            .filter(|name| !name.is_empty())
            .and_then(|name| {
                std::path::Path::new(name)
                    .file_stem()
                    .and_then(|stem| stem.to_str())
            })
            .unwrap_or("image");

        format!("{stem}.{ext}")
    }

    pub fn dimensions(&self) -> Option<(u32, u32)> {
        if let Format::Raster(_) = self.format {
            self.handle.to_vec().and_then(|bytes| {
                image::load_from_memory(&bytes)
                    .ok()
                    .map(|decoded| decoded.dimensions())
            })
        } else {
            None
        }
    }
}
