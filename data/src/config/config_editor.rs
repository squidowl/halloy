use serde::{Deserialize, Deserializer};
use serde_untagged::UntaggedEnumVisitor;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ConfigEditor {
    #[serde(deserialize_with = "deserialize_indent")]
    pub indent: Indent,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Indent {
    #[default]
    Tab,
    Spaces(usize),
}

fn deserialize_indent<'de, D>(deserializer: D) -> Result<Indent, D::Error>
where
    D: Deserializer<'de>,
{
    UntaggedEnumVisitor::new()
        .u32(|value| {
            if value > 0 && value <= 8 {
                Ok(Indent::Spaces(value as usize))
            } else {
                Err(serde::de::Error::invalid_value(
                    serde::de::Unexpected::Unsigned(u64::from(value)),
                    &"a positive integer from 1 to 8",
                ))
            }
        })
        .string(|string| match string {
            "tab" => Ok(Indent::Tab),
            _ => Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(string),
                &"\"tab\" or a positive integer from 1 to 8",
            )),
        })
        .deserialize(deserializer)
}
