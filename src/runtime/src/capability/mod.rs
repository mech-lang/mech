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

}