pub mod token;
pub mod kernel;
pub mod error;
pub mod basic;
pub mod capability;
pub mod filesystem;
mod grant;

pub use token::*;
pub use kernel::*;
pub use error::*;
pub use basic::*;
pub use capability::*;
pub use filesystem::*;
pub use grant::*;


// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use crate::*;
  use mech_core::MResult;
  use std::collections::{HashMap, HashSet};
  use std::sync::Arc;

  #[test]
  fn basic_capability_grant_and_check() {
    let subject = BasicSubject::new("task://1");
    let resource = BasicResource::new("db://users");

    let cap = BasicCapability::new(
      CapabilityId(1),
      &subject,
      &resource,
      [BasicOperation::read()],
    );

    let mut kernel = BasicCapabilityKernel::new();

    kernel
      .grant(CapabilityGrant::new(Arc::new(cap)))
      .unwrap();

    let request = CapabilityRequest::new(
      &subject,
      &BasicOperation::read(),
      &resource,
    );

    assert_eq!(kernel.check(&request).unwrap(), CapabilityId(1));
  }

  #[test]
  fn basic_capability_denies_wrong_operation() {
    let subject = BasicSubject::new("task://1");
    let resource = BasicResource::new("db://users");

    let cap = BasicCapability::new(
      CapabilityId(1),
      &subject,
      &resource,
      [BasicOperation::read()],
    );

    let mut kernel = BasicCapabilityKernel::new();

    kernel
      .grant(CapabilityGrant::new(Arc::new(cap)))
      .unwrap();

    let request = CapabilityRequest::new(
      &subject,
      &BasicOperation::write(),
      &resource,
    );

    assert!(kernel.check(&request).is_err());
  }

  #[test]
  fn basic_capability_resource_prefix_allows_nested_resource() {
    let subject = BasicSubject::new("task://1");
    let root = BasicResource::new("db");
    let users = BasicResource::new("db://users");

    let cap = BasicCapability::new(
      CapabilityId(1),
      &subject,
      &root,
      [BasicOperation::read()],
    )
    .with_constraints(
      BasicConstraints::default()
        .with_resource_prefix("db://"),
    );

    let mut kernel = BasicCapabilityKernel::new();

    kernel
      .grant(CapabilityGrant::new(Arc::new(cap)))
      .unwrap();

    let request = CapabilityRequest::new(
      &subject,
      &BasicOperation::read(),
      &users,
    );

    assert_eq!(kernel.check(&request).unwrap(), CapabilityId(1));
  }

  #[test]
  fn revocation_blocks_use() {
    let subject = BasicSubject::new("task://1");
    let resource = BasicResource::new("db://users");

    let cap = BasicCapability::new(
      CapabilityId(1),
      &subject,
      &resource,
      [BasicOperation::read()],
    );

    let mut kernel = BasicCapabilityKernel::new();

    kernel
      .grant(CapabilityGrant::new(Arc::new(cap)))
      .unwrap();

    kernel
      .revoke(CapabilityRevocation::new(CapabilityId(1)))
      .unwrap();

    let request = CapabilityRequest::new(
      &subject,
      &BasicOperation::read(),
      &resource,
    );

    assert!(kernel.check(&request).is_err());
  }

  #[test]
  fn delegation_requires_delegable_source() {
    let subject = BasicSubject::new("task://1");
    let next_subject = BasicSubject::new("task://2");
    let resource = BasicResource::new("db://users");

    let cap = BasicCapability::new(
      CapabilityId(1),
      &subject,
      &resource,
      [BasicOperation::read()],
    );

    let mut kernel = BasicCapabilityKernel::new();

    kernel
      .grant(CapabilityGrant::new(Arc::new(cap)))
      .unwrap();

    let derivation = CapabilityDerivation::delegate(
      CapabilityId(1),
      CapabilityId(2),
      &subject,
      &next_subject,
    );

    assert!(kernel.derive_capability(derivation).is_err());
  }

  #[test]
  fn delegable_source_can_be_delegated() {
    let subject = BasicSubject::new("task://1");
    let next_subject = BasicSubject::new("task://2");
    let resource = BasicResource::new("db://users");

    let cap = BasicCapability::new(
      CapabilityId(1),
      &subject,
      &resource,
      [BasicOperation::read()],
    )
    .delegable(true);

    let mut kernel = BasicCapabilityKernel::new();

    kernel
      .grant(CapabilityGrant::new(Arc::new(cap)))
      .unwrap();

    let derivation = CapabilityDerivation::delegate(
      CapabilityId(1),
      CapabilityId(2),
      &subject,
      &next_subject,
    );

    assert_eq!(
      kernel.derive_capability(derivation).unwrap(),
      CapabilityId(2),
    );
  }

  #[test]
  fn attenuation_can_reduce_operations() {
    let subject = BasicSubject::new("task://1");
    let resource = BasicResource::new("db://users");

    let cap = BasicCapability::new(
      CapabilityId(1),
      &subject,
      &resource,
      [BasicOperation::read(), BasicOperation::write()],
    );

    let mut kernel = BasicCapabilityKernel::new();

    kernel
      .grant(CapabilityGrant::new(Arc::new(cap)))
      .unwrap();

    let derivation = CapabilityDerivation::attenuate(
      CapabilityId(1),
      CapabilityId(2),
      &subject,
    )
    .with_operations([":read"]);

    assert_eq!(
      kernel.derive_capability(derivation).unwrap(),
      CapabilityId(2),
    );

    let derived = kernel
      .get(CapabilityId(2))
      .unwrap()
      .expect("derived capability should exist");

    let read_request = CapabilityRequest::new(
      &subject,
      &BasicOperation::read(),
      &resource,
    );

    assert!(derived.check(&read_request).unwrap().allowed);

    let write_request = CapabilityRequest::new(
      &subject,
      &BasicOperation::write(),
      &resource,
    );

    assert!(!derived.check(&write_request).unwrap().allowed);
  }

  #[test]
  fn token_payload_is_deterministic() {
    let subject = BasicSubject::new("task://1");
    let issuer = BasicSubject::new("host://root");

    let a = BasicCapabilityToken::new(
      CapabilityId(10),
      &subject,
      &issuer,
      "key-1",
      100,
      vec![CapabilityId(2), CapabilityId(1)],
    );

    let b = BasicCapabilityToken::new(
      CapabilityId(10),
      &subject,
      &issuer,
      "key-1",
      100,
      vec![CapabilityId(1), CapabilityId(2)],
    );

    assert_eq!(a.signing_payload().unwrap(), b.signing_payload().unwrap());
  }

  #[test]
  fn max_uses_is_enforced_by_default_kernel() {
    let subject = BasicSubject::new("task://1");
    let resource = BasicResource::new("db://users");

    let cap = BasicCapability::new(
      CapabilityId(1),
      &subject,
      &resource,
      [BasicOperation::read()],
    )
    .with_constraints(BasicConstraints::default().with_max_uses(1));

    let mut kernel = BasicCapabilityKernel::new();
    kernel.grant(CapabilityGrant::new(Arc::new(cap))).unwrap();

    let request = CapabilityRequest::new(&subject, &BasicOperation::read(), &resource);

    assert_eq!(kernel.check(&request).unwrap(), CapabilityId(1));
    assert!(kernel.check(&request).is_err());
  }

  #[test]
  fn rollback_grant_bypasses_non_revocable_policy() {
    let mut kernel = BasicCapabilityKernel::new();
    let capability = BasicCapability::from_keys(
      CapabilityId(1),
      "task://1",
      "db://users",
      [":read"],
    )
    .revocable(false);
    let request = CapabilityRequest::from_keys("task://1", ":read", "db://users");
    let subject = BasicSubject::new("task://1");

    kernel
      .grant(CapabilityGrant::new(Arc::new(capability)))
      .unwrap();
    assert_eq!(kernel.check(&request).unwrap(), CapabilityId(1));

    kernel.rollback_grant(CapabilityId(1)).unwrap();

    assert!(kernel.get(CapabilityId(1)).unwrap().is_none());
    assert!(!kernel
      .list_for_subject(&subject)
      .unwrap()
      .contains(&CapabilityId(1)));
    assert!(kernel.check(&request).is_err());
  }

  #[test]
  fn rollback_grant_clears_use_accounting() {
    let mut kernel = BasicCapabilityKernel::new();
    let request = CapabilityRequest::from_keys("task://1", ":read", "db://users");

    let capability = BasicCapability::from_keys(
      CapabilityId(1),
      "task://1",
      "db://users",
      [":read"],
    )
    .with_constraints(BasicConstraints::default().with_max_uses(1));
    kernel
      .grant(CapabilityGrant::new(Arc::new(capability)))
      .unwrap();
    assert_eq!(kernel.check(&request).unwrap(), CapabilityId(1));

    kernel.rollback_grant(CapabilityId(1)).unwrap();

    let replacement = BasicCapability::from_keys(
      CapabilityId(1),
      "task://1",
      "db://users",
      [":read"],
    )
    .with_constraints(BasicConstraints::default().with_max_uses(1));
    kernel
      .grant(CapabilityGrant::new(Arc::new(replacement)))
      .unwrap();
    assert_eq!(kernel.check(&request).unwrap(), CapabilityId(1));
  }

  #[test]
  fn basic_kernel_enforces_authority_allowlists() {
    let subject = BasicSubject::new("task://1");
    let resource = BasicResource::new("db://users");
    let mut kernel = BasicCapabilityKernel::new();
    for id in [CapabilityId(1), CapabilityId(2)] {
      kernel
        .grant(CapabilityGrant::new(Arc::new(
          BasicCapability::new(
            id,
            &subject,
            &resource,
            [BasicOperation::read()],
          ),
        )))
        .unwrap();
    }
    let request = CapabilityRequest::new(
      &subject,
      &BasicOperation::read(),
      &resource,
    );

    assert_eq!(
      kernel
        .check_scoped(
          &request,
          &RuntimeAuthorityScope::allow_list([CapabilityId(2)]),
        )
        .unwrap(),
      CapabilityId(2),
    );
    assert!(
      kernel
        .check_scoped(
          &request,
          &RuntimeAuthorityScope::allow_list([]),
        )
        .is_err(),
    );
  }

  #[test]
  fn rollback_grant_removes_descendants_and_graph_indexes() {
    let mut kernel = BasicCapabilityKernel::new();
    let parent_subject = BasicSubject::new("task://1");
    let child_subject = BasicSubject::new("task://2");

    let parent = BasicCapability::from_keys(
      CapabilityId(1),
      parent_subject.key(),
      "db://users",
      [":read"],
    )
    .delegable(true);
    kernel
      .grant(CapabilityGrant::new(Arc::new(parent)))
      .unwrap();
    kernel
      .derive_capability(CapabilityDerivation::delegate(
        CapabilityId(1),
        CapabilityId(2),
        &parent_subject,
        &child_subject,
      ))
      .unwrap();

    kernel.rollback_grant(CapabilityId(1)).unwrap();

    assert!(kernel.get(CapabilityId(1)).unwrap().is_none());
    assert!(kernel.get(CapabilityId(2)).unwrap().is_none());
    assert!(kernel.list_for_subject(&parent_subject).unwrap().is_empty());
    assert!(kernel.list_for_subject(&child_subject).unwrap().is_empty());

    let replacement = BasicCapability::from_keys(
      CapabilityId(1),
      parent_subject.key(),
      "db://users",
      [":read"],
    )
    .delegable(true);
    kernel
      .grant(CapabilityGrant::new(Arc::new(replacement)))
      .unwrap();
    assert_eq!(
      kernel
        .derive_capability(CapabilityDerivation::delegate(
          CapabilityId(1),
          CapabilityId(2),
          &parent_subject,
          &child_subject,
        ))
        .unwrap(),
      CapabilityId(2),
    );
  }

  #[test]
  fn rollback_grant_is_idempotent() {
    let mut kernel = BasicCapabilityKernel::new();
    let capability = BasicCapability::from_keys(
      CapabilityId(1),
      "task://1",
      "db://users",
      [":read"],
    );
    kernel
      .grant(CapabilityGrant::new(Arc::new(capability)))
      .unwrap();

    assert!(kernel.rollback_grant(CapabilityId(1)).is_ok());
    assert!(kernel.rollback_grant(CapabilityId(1)).is_ok());
  }

  #[test]
  fn constrained_capability_denies_missing_metrics() {
    let subject = BasicSubject::new("task://1");
    let resource = BasicResource::new("db://users");
    let cap = BasicCapability::new(
      CapabilityId(1),
      &subject,
      &resource,
      [BasicOperation::read()],
    )
    .with_constraints(
      BasicConstraints::default()
        .with_max_bytes(10)
        .with_max_items(2)
        .with_max_duration_ms(100),
    );

    let request = CapabilityRequest::new(&subject, &BasicOperation::read(), &resource);
    assert!(!cap.check(&request).unwrap().allowed);

    let request = request.with_context(CapabilityContext::local().with_bytes(10));
    assert!(!cap.check(&request).unwrap().allowed);

    let request = request.with_context(
      CapabilityContext::local().with_bytes(10).with_items(2),
    );
    assert!(!cap.check(&request).unwrap().allowed);

    let request = request.with_context(
      CapabilityContext::local()
        .with_bytes(10)
        .with_items(2)
        .with_duration_ms(100),
    );
    assert!(cap.check(&request).unwrap().allowed);
  }

  #[test]
  fn attenuation_cannot_relax_local_only() {
    let subject = BasicSubject::new("task://1");
    let resource = BasicResource::new("db://users");

    let cap = BasicCapability::new(
      CapabilityId(1),
      &subject,
      &resource,
      [BasicOperation::read()],
    )
    .with_constraints(BasicConstraints::default().local_only());

    let mut kernel = BasicCapabilityKernel::new();
    kernel.grant(CapabilityGrant::new(Arc::new(cap))).unwrap();

    let derivation = CapabilityDerivation::attenuate(
      CapabilityId(1),
      CapabilityId(2),
      &subject,
    )
    .with_constraints(BasicConstraints::default());

    assert!(kernel.derive_capability(derivation).is_err());
  }

  #[test]
  fn attenuation_cannot_increase_limits() {
    let subject = BasicSubject::new("task://1");
    let resource = BasicResource::new("db://users");

    let cap = BasicCapability::new(
      CapabilityId(1),
      &subject,
      &resource,
      [BasicOperation::read()],
    )
    .with_constraints(BasicConstraints::default().with_max_bytes(10));

    let mut kernel = BasicCapabilityKernel::new();
    kernel.grant(CapabilityGrant::new(Arc::new(cap))).unwrap();

    let derivation = CapabilityDerivation::attenuate(
      CapabilityId(1),
      CapabilityId(2),
      &subject,
    )
    .with_constraints(BasicConstraints::default().with_max_bytes(20));

    assert!(kernel.derive_capability(derivation).is_err());
  }

  #[test]
  fn parent_revocation_revokes_descendant_by_default() {
    let subject = BasicSubject::new("task://1");
    let next_subject = BasicSubject::new("task://2");
    let resource = BasicResource::new("db://users");

    let cap = BasicCapability::new(
      CapabilityId(1),
      &subject,
      &resource,
      [BasicOperation::read()],
    )
    .delegable(true);

    let mut kernel = BasicCapabilityKernel::new();
    kernel.grant(CapabilityGrant::new(Arc::new(cap))).unwrap();

    let derivation = CapabilityDerivation::delegate(
      CapabilityId(1),
      CapabilityId(2),
      &subject,
      &next_subject,
    );
    kernel.derive_capability(derivation).unwrap();

    kernel
      .revoke(CapabilityRevocation::new(CapabilityId(1)))
      .unwrap();

    assert!(kernel.is_revoked(CapabilityId(1)).unwrap());
    assert!(kernel.is_revoked(CapabilityId(2)).unwrap());
  }

  fn pending_use_fixture(
    max_uses: u64,
  ) -> (
    BasicCapabilityKernel,
    CapabilityRequest,
    CapabilityId,
  ) {
    let subject = BasicSubject::new("task://pending");
    let resource = BasicResource::new("db://users");
    let id = CapabilityId(900);
    let capability = BasicCapability::new(
      id,
      &subject,
      &resource,
      [BasicOperation::read()],
    )
    .with_constraints(
      BasicConstraints::default().with_max_uses(max_uses),
    );
    let mut kernel = BasicCapabilityKernel::new();
    kernel
      .grant(CapabilityGrant::new(Arc::new(capability)))
      .unwrap();
    (
      kernel,
      CapabilityRequest::new(
        &subject,
        &BasicOperation::read(),
        &resource,
      ),
      id,
    )
  }

  #[test]
  fn pending_transactional_uses_enforce_live_capability_limit() {
    let (kernel, request, id) = pending_use_fixture(1);
    assert_eq!(
      kernel
        .preview_check_excluding_with_pending_uses(
          &request,
          &HashSet::new(),
          &HashMap::new(),
        )
        .unwrap(),
      id,
    );
    assert!(kernel
      .preview_check_excluding_with_pending_uses(
        &request,
        &HashSet::new(),
        &HashMap::from([(id, 1)]),
      )
      .is_err());
    assert_eq!(kernel.successful_uses_for_test(id), 0);
  }

  #[test]
  fn committed_and_pending_capability_uses_are_combined() {
    let (mut kernel, request, id) = pending_use_fixture(2);
    kernel.apply_usage_delta(id, 1).unwrap();

    assert_eq!(
      kernel
        .preview_check_excluding_with_pending_uses(
          &request,
          &HashSet::new(),
          &HashMap::new(),
        )
        .unwrap(),
      id,
    );
    assert!(kernel
      .preview_check_excluding_with_pending_uses(
        &request,
        &HashSet::new(),
        &HashMap::from([(id, 1)]),
      )
      .is_err());
    assert_eq!(kernel.successful_uses_for_test(id), 1);
  }

  #[test]
  fn pending_use_preview_keeps_exclusions_and_shared_forwarding() {
    let (kernel, request, id) = pending_use_fixture(2);
    assert!(kernel
      .preview_check_excluding_with_pending_uses(
        &request,
        &HashSet::from([id]),
        &HashMap::new(),
      )
      .is_err());

    let shared = SharedCapabilityKernel::from_kernel(kernel);
    assert_eq!(
      shared
        .preview_check_excluding_with_pending_uses(
          &request,
          &HashSet::new(),
          &HashMap::from([(id, 1)]),
        )
        .unwrap(),
      id,
    );
    assert_eq!(shared.successful_uses_for_test(id), 0);
  }

  #[derive(Debug)]
  struct LegacyPreviewKernel {
    selected: CapabilityId,
  }

  impl CapabilityKernel for LegacyPreviewKernel {
    fn grant(&mut self, _grant: CapabilityGrant) -> MResult<CapabilityId> {
      unimplemented!()
    }

    fn rollback_grant(&mut self, _capability: CapabilityId) -> MResult<()> {
      unimplemented!()
    }

    fn revoke(&mut self, _revocation: CapabilityRevocation) -> MResult<()> {
      unimplemented!()
    }

    fn check(
      &mut self,
      _request: &CapabilityRequest,
    ) -> MResult<CapabilityId> {
      unimplemented!()
    }

    fn preview_check_excluding(
      &self,
      _request: &CapabilityRequest,
      _excluded: &HashSet<CapabilityId>,
    ) -> MResult<CapabilityId> {
      Ok(self.selected)
    }

    fn get(
      &self,
      _id: CapabilityId,
    ) -> MResult<Option<Arc<dyn Capability>>> {
      unimplemented!()
    }

    fn list_for_subject(
      &self,
      _subject: &dyn Subject,
    ) -> MResult<Vec<CapabilityId>> {
      unimplemented!()
    }

    fn derive_capability(
      &mut self,
      _derivation: CapabilityDerivation,
    ) -> MResult<CapabilityId> {
      unimplemented!()
    }

    fn is_revoked(&self, _id: CapabilityId) -> MResult<bool> {
      unimplemented!()
    }
  }

  #[test]
  fn unsupported_kernel_fails_closed_for_pending_uses() {
    let (_, request, id) = pending_use_fixture(1);
    let kernel = LegacyPreviewKernel { selected: id };

    assert_eq!(
      kernel
        .preview_check_excluding_with_pending_uses(
          &request,
          &HashSet::new(),
          &HashMap::new(),
        )
        .unwrap(),
      id,
    );
    let error = kernel
      .preview_check_excluding_with_pending_uses(
        &request,
        &HashSet::new(),
        &HashMap::from([(id, 1)]),
      )
      .unwrap_err();
    assert_eq!(error.kind_name(), "TransactionStateUnsupported");
  }

  #[test]
  fn custom_kernel_fails_closed_for_authority_allowlists() {
    let (_, request, id) = pending_use_fixture(1);
    let mut kernel = LegacyPreviewKernel { selected: id };
    let scope = RuntimeAuthorityScope::allow_list([id]);

    assert_eq!(
      kernel
        .check_scoped(&request, &scope)
        .unwrap_err()
        .kind_name(),
      "TransactionStateUnsupported",
    );
    assert_eq!(
      kernel
        .preview_scoped_with_transaction(
          &request,
          &scope,
          &HashSet::new(),
          &HashMap::new(),
        )
        .unwrap_err()
        .kind_name(),
      "TransactionStateUnsupported",
    );
  }

}
