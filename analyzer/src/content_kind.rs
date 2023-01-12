use std::fmt::{Display, Write};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContentKind {
    Advertisement,
    Music,
    Talk,
    Unknown,
}

impl ContentKind {
    pub fn name(&self) -> &'static str {
        match self {
            ContentKind::Advertisement => "Advertisement",
            ContentKind::Music => "Music",
            ContentKind::Talk => "Talk",
            ContentKind::Unknown => "Unknown",
        }
    }
}
impl TryFrom<&str> for ContentKind {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Advertisement" => Ok(ContentKind::Advertisement),
            "Music" => Ok(ContentKind::Music),
            "Talk" => Ok(ContentKind::Talk),
            "Unknown" => Ok(ContentKind::Unknown),
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
