use super::{BrowserAuthority, BrowserCapabilityRequest, BrowserHostError};

#[derive(Clone, Debug, Default)]
pub struct BrowserHost {
    authority: BrowserAuthority,
}

impl BrowserHost {
    pub fn new(authority: BrowserAuthority) -> Self {
        Self { authority }
    }

    pub fn deny_by_default() -> Self {
        Self::default()
    }

    pub fn authority(&self) -> &BrowserAuthority {
        &self.authority
    }

    pub fn authority_mut(&mut self) -> &mut BrowserAuthority {
        &mut self.authority
    }

    pub fn check(&self, request: BrowserCapabilityRequest) -> Result<(), BrowserHostError> {
        self.authority
            .check(&request)
            .map_err(BrowserHostError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::{BrowserCapabilityGrant, BrowserOperation, BrowserResource};

    #[test]
    fn browser_host_defaults_to_empty_authority() {
        let host = BrowserHost::deny_by_default();
        assert!(host.authority().grants().is_empty());
        assert!(host
            .check(BrowserCapabilityRequest::Clipboard {
                operation: BrowserOperation::Read,
            })
            .is_err());
    }

    #[test]
    fn browser_host_delegates_checks_to_authority() {
        let authority = BrowserAuthority::new([BrowserCapabilityGrant::new(
            BrowserResource::Clipboard,
            [BrowserOperation::Write],
        )]);
        let host = BrowserHost::new(authority);
        assert_eq!(
            host.check(BrowserCapabilityRequest::Clipboard {
                operation: BrowserOperation::Write,
            }),
            Ok(())
        );
    }
}
