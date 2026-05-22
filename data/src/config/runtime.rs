use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Runtime {
    pub backend: Backend,
    pub vsync: bool,
    pub antialiasing: bool,
}

impl Default for Runtime {
    fn default() -> Self {
        Self {
            backend: Backend::default(),
            vsync: true,
            antialiasing: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HardwareApi {
    #[default]
    Best,
    Vulkan,
    Metal,
    DirectX12,
    OpenGL,
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
