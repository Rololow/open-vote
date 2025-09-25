use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// Canonical categories for laws. Serialized/deserialized as a single string.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LawCategory {
    Constitutional,
    Civil,
    Criminal,
    Economic,
    Finance,
    Governance,
    Digital,
    /// Any category string that doesn't match the known set
    Other(String),
}

impl LawCategory {
    /// Create from a category string (case-insensitive, trims whitespace).
    /// Accepts both English canonical names and a few common French labels.
    pub fn from_str(input: &str) -> Self {
        let s = input.trim();
        let lower = s.to_ascii_lowercase();
        match lower.as_str() {
            // English
            "constitutional" => LawCategory::Constitutional,
            "civil" => LawCategory::Civil,
            "criminal" => LawCategory::Criminal,
            "economic" | "economy" => LawCategory::Economic,
            "finance" | "financial" => LawCategory::Finance,
            "governance" | "government" => LawCategory::Governance,
            "digital" | "technology" | "it" => LawCategory::Digital,
            // French common labels
            "constitutionnel" => LawCategory::Constitutional,
            "pénal" | "penal" => LawCategory::Criminal,
            "économique" | "economique" => LawCategory::Economic,
            "finances" => LawCategory::Finance,
            "gouvernance" => LawCategory::Governance,
            "numérique" | "numerique" => LawCategory::Digital,
            _ => LawCategory::Other(s.to_string()),
        }
    }

    /// Return a canonical, English, kebab-case-like string for the category.
    pub fn as_str(&self) -> &str {
        match self {
            LawCategory::Constitutional => "Constitutional",
            LawCategory::Civil => "Civil",
            LawCategory::Criminal => "Criminal",
            LawCategory::Economic => "Economic",
            LawCategory::Finance => "Finance",
            LawCategory::Governance => "Governance",
            LawCategory::Digital => "Digital",
            LawCategory::Other(s) => s.as_str(),
        }
    }

    /// CSS-friendly slug for the category (lowercase, hyphen-separated).
    pub fn slug(&self) -> String {
        self.as_str()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else if c.is_whitespace() { '-' } else { '-' })
            .collect()
    }

    /// Human-friendly label (preserves custom names for Other).
    pub fn label(&self) -> String {
        match self {
            LawCategory::Other(s) => s.clone(),
            _ => self.as_str().to_string(),
        }
    }
}

impl fmt::Display for LawCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

impl Serialize for LawCategory {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for LawCategory {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(LawCategory::from_str(&s))
    }
}
