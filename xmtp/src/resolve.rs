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
    fn parse_eth_address() {
        let addr = "0x1234567890abcdef1234567890abcdef12345678";
        assert_eq!(
            Recipient::parse(addr),
            Recipient::Address(addr.to_lowercase())
        );
    }

    #[test]
    fn parse_eth_address_uppercase() {
        let addr = "0xABCDEF1234567890ABCDEF1234567890ABCDEF12";
        assert_eq!(
            Recipient::parse(addr),
            Recipient::Address(addr.to_lowercase())
        );
    }

    #[test]
    fn parse_eth_address_trimmed() {
        let addr = "  0x1234567890abcdef1234567890abcdef12345678  ";
        assert_eq!(
            Recipient::parse(addr),
            Recipient::Address(addr.trim().to_lowercase())
        );
    }

    #[test]
    fn parse_ens_name() {
        assert_eq!(
            Recipient::parse("vitalik.eth"),
            Recipient::Ens("vitalik.eth".into())
        );
    }

    #[test]
    fn parse_ens_subdomain() {
        assert_eq!(
            Recipient::parse("sub.name.eth"),
            Recipient::Ens("sub.name.eth".into())
        );
    }

    #[test]
    fn parse_inbox_id() {
        let id = "abc123deadbeef";
        assert_eq!(Recipient::parse(id), Recipient::InboxId(id.into()));
    }

    #[test]
    fn parse_short_hex_is_inbox_id() {
        let short = "0x1234";
        assert_eq!(Recipient::parse(short), Recipient::InboxId(short.into()));
    }

    #[test]
    fn display_roundtrip() {
        let addr = "0x1234567890abcdef1234567890abcdef12345678";
        let r = Recipient::parse(addr);
        assert_eq!(r.to_string(), addr.to_lowercase());
    }

    #[test]
    fn from_str_trait() {
        let r: Recipient = "vitalik.eth".into();
        assert_eq!(r, Recipient::Ens("vitalik.eth".into()));
    }

    #[test]
    fn from_string_trait() {
        let r: Recipient = String::from("vitalik.eth").into();
        assert_eq!(r, Recipient::Ens("vitalik.eth".into()));
    }

    #[test]
    fn from_account_identifier() {
        use crate::types::{AccountIdentifier, IdentifierKind};
        let id = AccountIdentifier {
            address: "0xabc".into(),
            kind: IdentifierKind::Ethereum,
        };
        assert_eq!(Recipient::from(id), Recipient::Address("0xabc".into()));
    }
}
