use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Runtime {
    pub backend: Backend,
    pub power_preference: PowerPreference,
    pub vsync: bool,
    pub antialiasing: bool,
    pub metrics_hinting: bool,
}

impl Default for Runtime {
    fn default() -> Self {
        Self {
            backend: Backend::default(),
            power_preference: PowerPreference::default(),
            vsync: true,
            antialiasing: false,
            metrics_hinting: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PowerPreference {
    #[default]
    None,
    LowPower,
    HighPerformance,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HardwareApi {
    #[default]
    Best,
    Vulkan,
    Metal,
    #[serde(alias = "direct-x12")]
    DirectX12,
    #[serde(alias = "open-gl")]
    OpenGL,
    #[serde(alias = "web-gpu")]
    WebGPU,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Backend {
    #[default]
    Best,
    Hardware(HardwareApi),
    Software,
}

impl<'de> Deserialize<'de> for Backend {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "kebab-case")]
        enum Value {
            Best,
            Hardware,
            Software,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "kebab-case")]
        enum Detailed {
            Hardware(HardwareApi),
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Data {
            Value(Value),
            Detailed(Detailed),
        }

        match Data::deserialize(deserializer)? {
            Data::Value(Value::Best) => Ok(Self::Best),
            Data::Value(Value::Hardware) => {
                Ok(Self::Hardware(HardwareApi::Best))
            }
            Data::Value(Value::Software) => Ok(Self::Software),
            Data::Detailed(Detailed::Hardware(api)) => Ok(Self::Hardware(api)),
        }
    }
}
