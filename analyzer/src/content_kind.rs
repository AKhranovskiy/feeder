use std::fmt::{Display, Write};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContentKind {
    Advertisement,
    Music,
    Talk,
    Unknown,
}

impl ContentKind {
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Advertisement => "Advertisement",
            Self::Music => "Music",
            Self::Talk => "Talk",
            Self::Unknown => "Unknown",
        }
    }
}
impl TryFrom<&str> for ContentKind {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Advertisement" => Ok(Self::Advertisement),
            "Music" => Ok(Self::Music),
            "Talk" => Ok(Self::Talk),
            "Unknown" => Ok(Self::Unknown),
            _ => anyhow::bail!("Unknown content kind: {value}"),
        }
    }
}

impl Display for ContentKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            f.write_str(self.name())
        } else {
            f.write_char(self.name().chars().next().unwrap())
        }
    }
}
