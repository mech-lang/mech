#![cfg(feature = "provider")]

use std::sync::{Arc, Mutex};

use mech_browser::{
    BROWSER_DOM_PROVIDER_URI, BrowserAuthority, BrowserCapabilityGrant, BrowserDomBackend,
    BrowserDomManifestEntry, BrowserDomPath, BrowserDomProperty, BrowserDomScope, BrowserOperation,
    BrowserResource, BrowserResourceProvider,
};
use mech_core::{
    EffectContract, EffectDeliveryPolicy, ExternalInteraction, IdempotencyRequirement, LegacyValue,
    MResult, ObservationContract, ObservationReplayPolicy, Ref,
};
use mech_runtime::{
    PreparedRuntimeEffect, RuntimeCapabilityOperation, RuntimeResourceProvider,
    RuntimeResourceReadRequest, RuntimeResourceWriteIntent, RuntimeResourceWritePreflightRequest,
    RuntimeResourceWriteRequest,
};

#[derive(Debug, Clone)]
struct TestDomBackend;

impl BrowserDomBackend for TestDomBackend {
    fn read_dom_string(
        &self,
        _entry: &BrowserDomManifestEntry,
        _requested_path: &BrowserDomPath,
    ) -> MResult<String> {
        Ok(String::new())
    }

    fn write_dom_string(
        &mut self,
        _entry: &BrowserDomManifestEntry,
        _requested_path: &BrowserDomPath,
        _value: &str,
    ) -> MResult<()> {
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
struct RecordingDomBackend {
    reads: Arc<std::sync::atomic::AtomicUsize>,
    writes: Arc<Mutex<Vec<String>>>,
}

impl BrowserDomBackend for RecordingDomBackend {
    fn read_dom_string(
        &self,
        _entry: &BrowserDomManifestEntry,
        _requested_path: &BrowserDomPath,
    ) -> MResult<String> {
        self.reads.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(String::new())
    }

    fn write_dom_string(
        &mut self,
        _entry: &BrowserDomManifestEntry,
        _requested_path: &BrowserDomPath,
        value: &str,
    ) -> MResult<()> {
        self.writes.lock().unwrap().push(value.to_string());
        Ok(())
    }
}

fn read_request() -> RuntimeResourceReadRequest {
    RuntimeResourceReadRequest {
        base_uri: BROWSER_DOM_PROVIDER_URI.to_string(),
        path: "body/header/title".to_string(),
        context_name: "ui".to_string(),
    }
}

fn authority() -> BrowserAuthority {
    let selector = BrowserDomScope::new("#title").unwrap();
    let mut authority = BrowserAuthority::default();
    authority.bind_dom_path(BrowserDomManifestEntry::new(
        BrowserDomPath::new("body/header/title").unwrap(),
        selector.clone(),
        BrowserDomProperty::Text,
        [BrowserOperation::Read, BrowserOperation::Write],
    ));
    authority.grant(BrowserCapabilityGrant::new(
        BrowserResource::Dom(selector),
        [BrowserOperation::Read, BrowserOperation::Write],
    ));
    authority
}

#[test]
fn default_browser_provider_keeps_legacy_dom_base() {
    let provider = BrowserResourceProvider::new(authority(), TestDomBackend);
    let bases = provider.base_uris();
    assert!(bases.iter().any(|base| base == BROWSER_DOM_PROVIDER_URI));
    assert!(bases.iter().any(|base| base == "browser://dom/"));
    assert!(bases.iter().any(|base| base == "browser://browser/dom"));
}

#[test]
fn instance_browser_provider_advertises_instance_dom_base() {
    let provider = BrowserResourceProvider::for_instance("browser", authority(), TestDomBackend);
    assert!(
        provider
            .base_uris()
            .iter()
            .any(|base| base == "browser://browser/dom")
    );
}

#[test]
fn default_browser_provider_preflights_legacy_dom_base() {
    let provider = BrowserResourceProvider::new(authority(), TestDomBackend);
    provider
        .preflight_write(RuntimeResourceWritePreflightRequest {
            base_uri: BROWSER_DOM_PROVIDER_URI.to_string(),
            path: "body/header/title".to_string(),
            context_name: "ui".to_string(),
            operation: RuntimeCapabilityOperation::Write,
            intent: RuntimeResourceWriteIntent::Assign,
        })
        .unwrap();
}

#[test]
fn browser_dom_declares_snapshot_observation_and_assign_effect_semantics() {
    let provider = BrowserResourceProvider::new(authority(), TestDomBackend);
    assert!(matches!(
        provider.semantic_read_contract().unwrap().interaction,
        ExternalInteraction::Observation(ObservationContract {
            replay: ObservationReplayPolicy::CaptureAsInputFact,
        }),
    ));
    assert!(matches!(
        provider
            .semantic_write_contract(RuntimeResourceWriteIntent::Assign)
            .unwrap()
            .interaction,
        ExternalInteraction::Effect(EffectContract {
            delivery: EffectDeliveryPolicy::AtMostOnce,
            idempotency: IdempotencyRequirement::NotRequired,
        }),
    ));
    assert!(
        provider
            .semantic_write_contract(RuntimeResourceWriteIntent::Send)
            .is_none()
    );
    assert!(!provider.observation_requires_input_driver(&read_request()));
}

#[test]
fn browser_dom_planning_validates_without_touching_the_backend() {
    let backend = RecordingDomBackend::default();
    let observed = backend.clone();
    let provider = BrowserResourceProvider::new(authority(), backend);

    assert_eq!(
        provider.plan_read(read_request()).unwrap(),
        LegacyValue::String(Ref::new(String::new())),
    );
    provider
        .plan_write(RuntimeResourceWriteRequest {
            base_uri: BROWSER_DOM_PROVIDER_URI.to_string(),
            path: "body/header/title".to_string(),
            context_name: "ui".to_string(),
            operation: RuntimeCapabilityOperation::Write,
            value: LegacyValue::String(Ref::new("planned".to_string())),
            intent: RuntimeResourceWriteIntent::Assign,
        })
        .unwrap();

    assert_eq!(observed.reads.load(std::sync::atomic::Ordering::SeqCst), 0,);
    assert!(observed.writes.lock().unwrap().is_empty());
}

#[test]
fn browser_dom_write_is_deferred_until_delivery() {
    let backend = RecordingDomBackend::default();
    let observed = backend.clone();
    let provider = BrowserResourceProvider::new(authority(), backend);
    let effect = provider
        .prepare_write(RuntimeResourceWriteRequest {
            base_uri: BROWSER_DOM_PROVIDER_URI.to_string(),
            path: "body/header/title".to_string(),
            context_name: "ui".to_string(),
            operation: RuntimeCapabilityOperation::Write,
            value: LegacyValue::String(Ref::new("deferred".to_string())),
            intent: RuntimeResourceWriteIntent::Assign,
        })
        .unwrap();

    assert!(observed.writes.lock().unwrap().is_empty());
    match effect {
        PreparedRuntimeEffect::AfterCommit(mut effect) => effect.deliver().unwrap(),
        effect => panic!("expected browser after-commit effect, got {effect:?}"),
    }
    assert_eq!(
        *observed.writes.lock().unwrap(),
        vec!["deferred".to_string()],
    );
}
