//! Unified recipient resolution for XMTP messaging.
//!
//! [`Recipient`] represents any identity the SDK can resolve to an XMTP inbox:
//! Ethereum addresses, inbox IDs, ENS names, and future identity types.
//!
//! [`Resolver`] is a pluggable trait for external name resolution (ENS, Lens, etc.).

use crate::error::Result;
use crate::types::IdentifierKind;

/// A message recipient — any form of identity the SDK can resolve.
///
/// Use [`Recipient::parse`] or `From<&str>` for automatic detection:
///
/// - `0x` + 40 hex chars → [`Address`](Recipient::Address)
/// - Contains `.` → [`Ens`](Recipient::Ens)
/// - Otherwise → [`InboxId`](Recipient::InboxId)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Recipient {
    /// Ethereum address (0x-prefixed, 42 chars).
    Address(String),
    /// XMTP inbox ID (hex string).
    InboxId(String),
    /// ENS name (e.g. `vitalik.eth`). Requires a [`Resolver`].
    Ens(String),
}

impl Recipient {
    /// Auto-detect the recipient type from a raw string.
    #[must_use]
    pub fn parse(input: &str) -> Self {
        let s = input.trim();
        if s.len() == 42
            && s.starts_with("0x")
            && s.as_bytes()
                .get(2..)
                .is_some_and(|b| b.iter().all(u8::is_ascii_hexdigit))
        {
            Self::Address(s.to_lowercase())
        } else if s.contains('.') {
            Self::Ens(s.to_owned())
        } else {
            Self::InboxId(s.to_owned())
        }
    }
}

impl From<&str> for Recipient {
    fn from(s: &str) -> Self {
        Self::parse(s)
    }
}

impl From<String> for Recipient {
    fn from(s: String) -> Self {
        Self::parse(&s)
    }
}

impl From<crate::types::AccountIdentifier> for Recipient {
    fn from(id: crate::types::AccountIdentifier) -> Self {
        match id.kind {
            IdentifierKind::Ethereum => Self::Address(id.address),
            IdentifierKind::Passkey => Self::InboxId(id.address),
        }
    }
}

impl std::fmt::Display for Recipient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Address(a) => f.write_str(a),
            Self::InboxId(id) => f.write_str(id),
            Self::Ens(name) => f.write_str(name),
        }
    }
}

/// Resolves external names (ENS, Lens, etc.) to Ethereum addresses and back.
///
/// Implement this trait to add custom identity resolution to the SDK.
/// Register via [`ClientBuilder::resolver`](crate::ClientBuilder::resolver).
pub trait Resolver: Send + Sync {
    /// Resolve a name to an Ethereum address (lowercase, 0x-prefixed).
    ///
    /// # Errors
    ///
    /// Returns [`XmtpError::Resolution`](crate::XmtpError::Resolution) if resolution fails.
    fn resolve(&self, name: &str) -> Result<String>;

    /// Reverse-resolve an Ethereum address to a human-readable name (e.g. ENS).
    ///
    /// Returns `Ok(None)` if no reverse record exists.
    ///
    /// # Errors
    ///
    /// Returns [`XmtpError::Resolution`](crate::XmtpError::Resolution) on network/lookup failure.
    fn reverse_resolve(&self, _address: &str) -> Result<Option<String>> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_eth_address_lowercase() {
        let addr = "0x1234567890abcdef1234567890abcdef12345678";
        assert_eq!(Recipient::parse(addr), Recipient::Address(addr.into()));
    }

    #[test]
    fn parse_eth_address_normalizes_case() {
        let mixed = "0xABCDef1234567890abcdef1234567890ABCDEF12";
        assert_eq!(
            Recipient::parse(mixed),
            Recipient::Address(mixed.to_lowercase())
        );
    }

    #[test]
    fn parse_trims_whitespace() {
        let padded = "  0x1234567890abcdef1234567890abcdef12345678  ";
        assert!(matches!(Recipient::parse(padded), Recipient::Address(_)));
    }

    #[test]
    fn parse_42_char_non_hex_is_not_address() {
        let bad = "0xZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ";
        assert_eq!(Recipient::parse(bad), Recipient::InboxId(bad.into()));
    }

    #[test]
    fn parse_short_0x_is_inbox_id() {
        assert_eq!(
            Recipient::parse("0x1234"),
            Recipient::InboxId("0x1234".into())
        );
    }

    #[test]
    fn parse_empty_string() {
        assert_eq!(Recipient::parse(""), Recipient::InboxId(String::new()));
    }

    #[test]
    fn parse_ens_name() {
        assert_eq!(
            Recipient::parse("vitalik.eth"),
            Recipient::Ens("vitalik.eth".into())
        );
    }

    #[test]
    fn parse_plain_string_is_inbox_id() {
        assert_eq!(
            Recipient::parse("abc123deadbeef"),
            Recipient::InboxId("abc123deadbeef".into())
        );
    }

    #[test]
    fn from_account_identifier_ethereum() {
        use crate::types::{AccountIdentifier, IdentifierKind};
        let id = AccountIdentifier {
            address: "0xabc".into(),
            kind: IdentifierKind::Ethereum,
        };
        assert_eq!(Recipient::from(id), Recipient::Address("0xabc".into()));
    }

    #[test]
    fn from_account_identifier_passkey() {
        use crate::types::{AccountIdentifier, IdentifierKind};
        let id = AccountIdentifier {
            address: "pk_123".into(),
            kind: IdentifierKind::Passkey,
        };
        assert_eq!(Recipient::from(id), Recipient::InboxId("pk_123".into()));
    }
}
