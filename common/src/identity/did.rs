use serde::{Serialize, Deserialize};

/// Représentation interne d'un DID supporté (Phase 2 minimaliste).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Did(pub String);

impl Did {
    pub fn as_str(&self) -> &str { &self.0 }
    pub fn method(&self) -> Option<&str> {
        self.0.split(':').nth(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_method() {
        let d = Did("did:key:zABCDEFG".into());
        assert_eq!(d.method(), Some("key"));
    }
}
