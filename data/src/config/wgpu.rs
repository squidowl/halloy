use std::fmt;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct Wgpu {
    pub backend: WgpuBackend,
    pub power_pref: WgpuPowerPref,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum WgpuBackend {
    #[default]
    Auto,
    Vulkan,
    Metal,
    #[serde(rename = "dx12")]
    Dx12,
    OpenGL,
}

impl fmt::Display for WgpuBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            WgpuBackend::Auto => "auto",
            WgpuBackend::Vulkan => "vulkan",
            WgpuBackend::Metal => "metal",
            WgpuBackend::Dx12 => "dx12",
            WgpuBackend::OpenGL => "gl",
        };

        write!(f, "{s}")
    }
}

impl WgpuBackend {
    pub fn to_wgpu_backends(self) -> iced::wgpu::Backends {
        match self {
            WgpuBackend::Auto => default_wgpu_backend(),

            WgpuBackend::Vulkan => {
                #[cfg(any(target_os = "linux", target_os = "windows"))]
                {
                    iced::wgpu::Backends::VULKAN
                }
                #[cfg(not(any(target_os = "linux", target_os = "windows")))]
                {
                    default_wgpu_backend()
                }
            }

            WgpuBackend::Metal => {
                #[cfg(target_os = "macos")]
                {
                    iced::wgpu::Backends::METAL
                }
                #[cfg(not(target_os = "macos"))]
                {
                    default_wgpu_backend()
                }
            }

            WgpuBackend::Dx12 => {
                #[cfg(target_os = "windows")]
                {
                    iced::wgpu::Backends::DX12
                }
                #[cfg(not(target_os = "windows"))]
                {
                    default_wgpu_backend()
                }
            }

            WgpuBackend::OpenGL => {
                #[cfg(any(target_os = "linux", target_os = "windows"))]
                {
                    iced::wgpu::Backends::GL
                }
            }
        }
    }
}

pub fn default_wgpu_backend() -> iced::wgpu::Backends {
    if cfg!(target_os = "windows") {
        iced::wgpu::Backends::VULKAN
    } else if cfg!(target_os = "macos") {
        iced::wgpu::Backends::METAL
    } else if cfg!(any(target_os = "linux", target_os = "android")) {
        iced::wgpu::Backends::VULKAN
    } else {
        iced::wgpu::Backends::all()
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum WgpuPowerPref {
    #[default]
    NotSet,

    Low,
    High,
}

impl fmt::Display for WgpuPowerPref {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            WgpuPowerPref::NotSet => "not set",

            WgpuPowerPref::Low => "low",
            WgpuPowerPref::High => "high",
        };

        write!(f, "{s}")
    }
}
