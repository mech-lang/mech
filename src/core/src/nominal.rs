//! Canonical nominal paths and durable nominal keys.

use crate::{NominalKey, NominalPathError, SemanticModelError};
use sha2::{Digest, Sha256};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, string::String, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, string::String, vec::Vec};

#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct CanonicalNominalPath {
    segments: Box<[String]>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum NominalKind {
    Atom = 0x01,
    Enum = 0x02,
}

impl CanonicalNominalPath {
    pub fn new(segments: impl Into<Box<[String]>>) -> Result<Self, SemanticModelError> {
        let segments = segments.into();
        if segments.is_empty() {
            return Err(SemanticModelError::InvalidNominalPath {
                segment: None,
                reason: NominalPathError::EmptyPath,
            });
        }
        for (index, segment) in segments.iter().enumerate() {
            let reason = if segment.is_empty() {
                Some(NominalPathError::EmptySegment)
            } else if segment == "." {
                Some(NominalPathError::DotSegment)
            } else if segment == ".." {
                Some(NominalPathError::DotDotSegment)
            } else if segment.as_bytes().contains(&0) {
                Some(NominalPathError::ContainsNul)
            } else {
                None
            };
            if let Some(reason) = reason {
                return Err(SemanticModelError::InvalidNominalPath {
                    segment: Some(index as u32),
                    reason,
                });
            }
        }
        Ok(Self { segments })
    }

    pub fn segments(&self) -> &[String] {
        &self.segments
    }

    pub fn canonical_bytes(&self) -> Box<[u8]> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(self.segments.len() as u32).to_le_bytes());
        for segment in &self.segments {
            bytes.extend_from_slice(&(segment.len() as u64).to_le_bytes());
            bytes.extend_from_slice(segment.as_bytes());
        }
        bytes.into_boxed_slice()
    }
}

impl NominalKey {
    pub fn from_path(kind: NominalKind, path: &CanonicalNominalPath) -> Self {
        let mut hash = Sha256::new();
        hash.update(b"mech-nominal-v1\0");
        hash.update([kind as u8]);
        hash.update(path.canonical_bytes());
        Self::from_bytes(hash.finalize().into())
    }
}
