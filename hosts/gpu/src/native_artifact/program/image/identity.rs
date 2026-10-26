//! Explicit program IR/ABI identity. No Debug or pointer serialization.
use mech_engine::native_artifact::{
    NativeIterationAction, NativeIterationTerminator, NativeIterationValueScope, NativeMatchAction,
    NativeMatchTerminator, NativeProgramArtifactPlan, NativeScalarKind, NativeSelectedTextJoin,
    NativeTextLineMatrixRegion, NativeTextLineRetentionFrontier,
};
use sha2::{Digest, Sha256};
// Identity construction runs once per descriptor, outside generated execution.
// Keep the streaming hash update out of its hundreds of serialization sites;
// this shares code without changing a single byte of the identity transcript.
#[inline(never)]
fn word(hash: &mut Sha256, value: u64) {
    hash.update(value.to_le_bytes());
}
#[inline(never)]
fn bytes(hash: &mut Sha256, value: &[u8]) {
    word(hash, value.len() as u64);
    hash.update(value);
}
fn selected_template(
    hash: &mut Sha256,
    template: &mech_engine::native_artifact::NativeProgramLibraryTemplate,
) {
    word(hash, template.origin().node().get() as u64);
    word(hash, template.origin().block().map_or(u64::MAX, u64::from));
    word(hash, template.origin().local().map_or(u64::MAX, u64::from));
    hash.update(template.program_revision().as_bytes());
    bytes(hash, template.primitive().name().as_bytes());
    let key = template.selected_key();
    word(hash, key.module_path.len() as u64);
    for part in &key.module_path {
        bytes(hash, part.as_bytes());
    }
    bytes(hash, key.operation_name.as_bytes());
    word(hash, template.inputs().len() as u64);
    for input in template.inputs() {
        bytes(hash, &input.canonical_bytes());
    }
    bytes(hash, &template.output().canonical_bytes());
    match template.contract().canonical_bytes() {
        Ok(encoded) => {
            hash.update([1]);
            bytes(hash, &encoded);
        }
        Err(_) => hash.update([0]), // opaque invalid plans never reach execution
    }
    use mech_core::ImplementationMemoryClass as M;
    let (tag, a, b) = match template.implementation_memory() {
        M::NoAdditionalScratch => (0, 0, 0),
        M::CloneInput { input } => (1, input, 0),
        M::CanonicalCloneInput { input } => (2, input, 0),
        M::AbiContiguousBridge { input, output } => (3, input, output),
        M::ExternalMarshalling => (4, 0, 0),
        M::MatrixSolve => (5, 0, 0),
        M::CanonicalFinalize => (6, 0, 0),
        M::CanonicalSortUnique => (7, 0, 0),
        M::PackedStringMatrixOrCanonicalFinalize => (8, 0, 0),
    };
    for number in [tag as u64, a as u64, b as u64] {
        word(hash, number);
    }
}
// IR25 pins deferred final counts, bounded scan progress and the unvisited
// EOF exit boundary before captured-source consumers acquire ownership.
// IR26 keeps a returned range open until its requested endpoint is physically
// available. The frontier itself is unchanged, so its old hash cannot safely
// authenticate Objects emitted before the corrected retention instructions.
// IR27 pins the optional sole-use Range producer/endpoints/inclusivity in the
// selected matrix-join proof. The three original LibraryCall visits remain.
// IR28 pins funded canonical child bindings in structural patterns.
// IR29 pins the complete same-plan retained-map candidate and word inventory
// carried by build-owned descriptors. It does not activate owner execution.
// IR30 pins the explicit owner boundary after borrowed/closed-loop storage;
// candidate words may not overlap those separately retained metadata lanes.
// IR31 selects conditional owner words and per-region live flags.
// IR32 pins non-escaping range/String projections used by retained map keys.
// IR33 pins single-entry native owner groups and precise interior error origins.
// IR34 pins acyclic native path frontiers and fused map-helper admission.
// IR35 binds a private storage pair and pinned String to closed map Tail bodies.
// IR36 admits a closed map Tail within an ordinary suspended parent loop.
// IR37 carries an invariant-width proof for generated private map storage.
// IR38 versions closed packed bodies into native SSA while preserving the
// original source CFG. Private op13 attests actual first-header selection.
// IR40 binds constructor-private parallel entries to immutable root captures
// and original collection result frontiers; linked children share ABI21.
// IR41 adds source-derived finite character domains to those same generated
// SSA bodies, retaining every canonical String map key and worker boundary.
// IR42 probes eight admitted control bytes with SIMD equality, retaining full
// packed-key verification, bounded wraparound and original source updates.
// IR43 substitutes only scalar guards/arithmetic proved by bounded exact
// current-traversal edge facts; source CFG, work charges and ABI are unchanged.
// IR44 selects a one-word key only when every admitted width fits the proved
// source domain; both representations retain the complete U64 source count.
// IR45 threads exact enum tags through a bounded product CFG. Initial/unknown
// tags still dispatch, and all original header debits and terminal visits run.
// IR46 emits the ABI24 folded-product hash in generated packed probes.
// IR47 synchronizes proved private helper read/write sets in packed SSA;
// the ABI, source work, finite charges and full exit/error spills are unchanged.
// IR48 adds a source-proved unsigned stop/range version, selected only when
// invariant widths/limits fit the actual pinned source. Original work remains.
// ABI25 binds private op14's validated, once-per-invocation selection receipt.
// IR49 binds complete-use frozen owners and private same-scope read helpers.
// IR50 carries source-proved current-key cache/presence facts within one
// closed traversal. New selections, headers and mutations invalidate aliases.
// IR51 removes only literal constant branches and unreachable continuations
// after native data optimization, before register allocation. Work is unchanged.
// IR52 isolates proved closed packed loops into local native functions; the
// identical inline body remains the fallback and finite work is unchanged.
// IR53 versions guarded calls and peels rolling-key adjacency into a
// source-proved steady loop. ABI27 binds the actual call-guard selection.
// IR54 cleans literal control flow before the single ordinary backend data
// optimization pass, avoiding repeated expression placement/rematerialization.
// IR55 moves native helper register preservation into an owned PreserveAll
// bridge. The original C ABI, source work envelope and finite limits remain.
// IR56 gives that bridge one caller-owned scratch-pointer argument instead of
// nine register/stack arguments. Original helper arguments and status remain.
// IR57 stores exact invocation-constant helper bindings once, outside the
// loop. Distinct function/context operands retain the ordinary native call.
// IR58 proves a single insertion per index+1 recurrence, checks the active
// entry's tally/index equality, and reuses that exact source progress. Only
// certified completed recurrence nodes establish steady nonempty storage.
// IR59 proves the stored-value bound of an empty, single-insertion source
// recurrence before omitting its exact read+1 source overflow checks.
// IR60 loads admitted AArch64 control groups directly into a 64-bit vector;
// integer fingerprints/empty masks and the eight-byte read extent remain.
// IR61 was an unpromoted cache-header experiment. IR62 binds immutable hash
// constants and a source-certified endpoint in owned caller stack storage.
// Rust helper ABI, packed frame layout and original work prices remain.
// IR63 was an unpromoted lookup-state experiment. IR64 separates a proved
// funded, present-entry source trace from its precise cold continuations.
// The original helper ABI, source operations and finite work prices remain.
// IR65 folds scalar expressions before constant control-edge cleanup inside
// helper-free funded functions; ordinary/checked functions keep IR64's policy.
// IR66 was an unpromoted immutable-snapshot experiment. IR67 versions the
// helper-free source graph by the actual private width, with bounded complete
// constant-width arms and the original generic fallback.
// IR68 moves the identical fuel test to each incoming header edge, keeping
// exact header continuations while retiring prior-iteration snapshot liveness.
// IR69 also admits productive traversal at those incoming edges. Terminal
// traversal returns before its original header/debit for exact outer replay.
// IR70 gives each incoming header edge its own precise cold snapshot, avoiding
// initial/completed-state joins while retaining the same source continuation.
// IR71 admits a direct present-entry continuation, a peeled first control
// group and raw first-match SIMD bits. Precise missing/error continuations,
// bounded later probing and original work prices remain.
// IR72 strip-mines the proved +1 source recurrence by its existing prepaid
// fuel and endpoint. A cold stack word reconstructs exact fuel on every exit;
// the hot source loop needs one stopping bound and no fuel counter mutation.
// IR73 binds that prepaid extent and cold fuel base as private call arguments,
// preventing loop rematerialization of their scalar preparation. Original
// source continuations, finite credit and external helper ABI remain unchanged.
// IR74 represents the proved adjacent read by a source-derived byte cursor.
// Opaque caller stop/credit pointers preserve the exact prepaid extent; cold
// continuations recover the original scalar counter and previous selection.
// IR75 reconstructs only source-proved affine scalar/frame values inside
// their precise cold continuations. Initial and completed cursor coordinates
// remain distinct; unknown values retain their original SSA storage.
// IR76 obtains the cold index bias from the owned frame at the snapshot,
// rather than sharing the hot source SSA expression and its live registers.
// Frame immutability, source recipes and original finite prices are unchanged.
// IR77 / ABI28 add an optional bounded complete-key lookup cache over the
// original hash table. Every key and source update remains; allocation, cache
// maintenance and growth refreshes are admitted under work policy66.
// IR78 separates admitted cached and ordinary hash funded functions. Both
// emit the original source graph; the caller binds actual cache/width guards,
// and code/stack admission includes both versions under unchanged prices.
// IR79 declares disjoint origin, borrowed-view and scalar-Tail metadata
// storage to native alias analysis. Calls retain full memory effects, and
// source checks, prices and continuation clearance remain unchanged.
// IR80 binds aligned private control-group placement to generated probes.
// Candidate payload addresses use control-bit byte offsets and an early group
// base. Frame layout, finite body prices and per-group charges are unchanged.
// IR81 / ABI29 read selected rows directly from existing LF offsets in a
// pinned one/two-chunk ASCII window. Other representations keep checked cells.
// IR82 composes a sole-use selected join with its source-derived byte map in
// initialized physical chunks. Original error/admission boundaries remain.
// IR86 carries a private same-account initialized String buffer across a
// complete-use-proved join/map chain, without a frozen intermediate or a
// second UTF-8 scan. Original admissions, finite prices and ABI30 remain.
// IR87 / ABI31 loads immutable selected-source spans and maps directly into
// checked output spare capacity. The original finite join/map prices remain;
// only a complete initialized-extent receipt permits publishing its length.
// IR88 / ABI32 prepare joined-map byte tables once at the owning host
// boundary, avoiding per-row u16-to-byte conversion. Original source proofs,
// geometry/extent receipts, work policy69 and finite prices are unchanged.
// IR89 reads only the length home for a borrowed Length action. The unused
// pointer load is absent; source checks, finite prices, ABI32 and policy69
// remain. No new memory-access promise is made.
// IR90 marks only owner-proved private origin/view/scalar-loan homes nontrapping.
// Mutable values, callback barriers, source bounds, ABI32 and policy69 remain;
// no readonly, alignment or arbitrary code-motion permission is introduced.
// IR91 / ABI33 share a prepared byte-table format between ordinary and joined
// mapping. A verified low-five-bit projection selects a two-vector leaf;
// incompatible maps retain all 128 cells. All source prices and owners remain.
// IR92 runs admitted callback-free scalar Tails with the maintained bounded
// tag graph and SSA aggregate/view homes. Original checkpoint, error, work
// prices, fallback and owner-retirement boundaries remain; ABI33 is unchanged.
// IR93 loads initialized candidate payloads beside hash keys in helper-free
// source loops. A callee-local valid bit preserves ordinary reload fallback;
// no runtime frame, resource price or library/storage ABI changes.
// IR94 reads the two initialized narrow-slot words together; wide slots retain
// separate full-width accesses. Runtime storage and original prices are unchanged.
pub const NATIVE_PROGRAM_TYPED_IR_VERSION: u64 = 94;
// Shared immutable schema arenas and the admitted two-phase entry protocol
// change work accounting. ABI8/IR5 also commit the exact generated checkpoint;
// neither phase may renew the invocation's cumulative ledger. Policy11 reuses
// exact live-geometry bindings within one invocation, retiring an old binding
// before a shape miss rebuilds it. Every visit measures and executes fresh Values.
// Policy12 replaces repeated managed lexical-scope discovery with exact
// descriptor-derived retirement tables, admitted once on the same session.
// Exact current facts may reuse admitted planning metadata (never values or
// permits). Scalar String capture uses canonical shared-data ownership bounds.
// Immutable String indices are weak-owner-bound, admitted independently, and
// counted in every fresh call/permit; compact boundaries require segmentation.
// Policy13/IR6 add an explicit selected U64 range source strategy. It preserves
// every candidate/error boundary and admits fixed source state instead of a
// nonexistent Matrix. ABI8's existing two-word collection helper wire is unchanged.
// Policy14/IR7 admit selected TextJoin append sinks. The elided Matrix is
// never initialized; its selected consumer alone publishes a canonical String.
// Policy15/IR8 add selected String histogram sinks and guarded query demands.
// No elided Matrix/Map/Set has a Value handle; scalar demand outputs retain
// canonical ownership and every generated producer visit remains unchanged.
// Policy16/IR9 add one private current-candidate unit-U64 String span. Both
// original selected visits remain; the actual source owner and index stay
// fully admitted until the existing sink consumes the checked byte range.
// Policy17 proves immutable selected query-stage associations once in the
// descriptor constructor, eliminating repeated deep equality and unused full
// binder reservations. Every original stage keeps fixed header work, dynamic
// generation/phase checks, comparisons and canonical scalar owner admission.
// ABI8's two-word helper transport and IR9's selected control proof are unchanged.
// Policy18/IR10 add a strict sole-use String element -> selected String map
// entry -> TextJoin strategy. Both original visits and complete owner pins
// remain; only nonexistent key/result allocations and their copies disappear.
// Canonical key planning/search and final append share maintained mechanics.
// Policy20 reuses paid immutable DATA footprints within the same invocation;
// every turn still validates its current finite limits and admission ledger.
// Policy21 composes recursive declared-schema conversion binding and staging
// with the collection strategies above. Present Matrix axes remain witnessed;
// every invocation keeps fresh admission and atomic publication obligations.
// Policy22/IR11 emit closed canonical window-query loops as native regions.
// Their finite byte/compare/instruction bound is charged before entry, not
// by visiting each operation. Existing activation, capture and atomic commit
// remain ONE round. Genuine String owners are loaned only across that body.
// Policy23 prepays a complete native histogram region and finite dense scratch.
// Policy24 prepays byte-map table/output and one complete String map loop.
// Policy25 admits private aggregate words once in the session's transport;
// no repeated canonical heap ownership is created for those unobservable values.
// Policy26 admits String owner tables once; bounded native equality charges
// the same cumulative comparison ledger and each generated visit covers its code.
// Policy27 funds the conditional selector elision; ordinary fallback and
// failure-only reconstruction retain the original selected maintained leaves.
// Policy28 funds one acyclic Tail traversal before native execution and
// reconciles its finite cumulative fuel before every callback/region exit.
// Policy29 preserves both original selection/join visits while charging their
// private metadata and complete canonical String construction on the same ledger.
// Policy30 admits private line geometry and its immutable String owner instead
// of copied line payloads. Original producer/length/selection visits remain;
// Unicode fallback inherits the same original call's already-spent allowance.
// Policy31 funds exact current-alias retirement and ownership handoff before
// a closed String map loop; copied/shared fallback stays on the same ledger.
// Policy32 funds header pauses/re-entry on the SAME cumulative account.
// Policy33 funds emitted retention min/range rules on that same account.
// Policy34 binds line-store/window debits to the existing shared program ledger.
// Policy35 funds EOF source admission, symbolic-count branches, scoped Unicode
// cell construction and scan-exit loan retirement on that same finite account.
// Policy36 admits actual fixed range geometry instead of allocating a dead
// numeric selector. Original-site shape/index errors and one ledger remain;
// less actual work/storage may let formerly budget-denied programs succeed.
// Policy37 admits one same-entry immutable representation-fact slot and its
// complete weak identity shell before use. It never retains source payloads
// or permissions; every call still pays the original full scan bound and
// validates its fresh owners, shapes and finite limits on the same ledger.
// Policy38 retains actual library hash-table updates for compiled ASCII
// windows instead of dense or query-only counters. Growth, collisions and
// ownership remain finite on the original invocation account. ABI13 binds the
// new private accumulator/callback loans and rejects preceding Object images.
// Policy39 produces rolling ASCII keys in the emitted loop and amortizes
// finite work on the same account. Capacity changes prepay remaining deltas;
// every window still updates the library table. ABI14 binds packed-key loans.
// Policy40 batches at most 128 initialized rolling keys in descriptor-admitted
// stack storage. ABI15 binds the private batch callback contract; every key
// retains its original table update and finite accounting.
// Policy41 admits canonical structural child measurement, copying and finalization
// on the same invocation ledger before publishing the bound value.
// Policy42 retains admitted node witnesses and funds per-slot node metadata;
// old Objects must not reuse the earlier snapshot/transport allocation prices.
// Policy43 funds each snapshot owner's account-wide node witness claim. The
// complete owner size, including that claim, remains part of byte admission.
// Policy44 removes the superseded per-slot node census. The account-wide
// snapshot claim still covers admitted owners, including caller-held roots.
// Policy45 retains the recursive binding node witness independently of byte
// reservations. Older Objects must not reuse the preceding binding layout price.
// Policy46 retains lazy map-owner session metadata through entry boundaries.
// Fixed caller geometry includes that state; table allocations remain lazy.
// Policy47 funds owner transport and retained session entry/call/exit operations.
// Policy48 maintains call-cache metadata totals at mutation boundaries rather
// than scanning every unrelated cache during each retained-map operation.
// Policy49 funds selected map-key metadata and genuine-owner projection lanes.
// Policy50 prepays straight-line native groups, retaining per-block canonical
// fallback prices and all library/ownership/loop admission boundaries.
// Policy51 admits an immutable outer caller census for closed map loops.
// Snapshot and cache holdings stay dynamic; each loop invocation owns its census.
// Policy52 prepays every native branch in an acyclic path and validates the
// next observable frontier. Map helpers admit themselves on the same ledger.
// Policy53 prepays bounded source-body batches on the same account used by
// private map storage. Checked growth and canonical publication remain funded.
// Policy54 retains one successful range shape in the invocation-owned map
// lane. Fresh endpoints and changed shapes are checked; full selector work
// and scratch remain prepaid, and private owner metadata remains accounted.
// Policy55 shares one setup/teardown of the native call's thread-local
// account, observer and cap addresses, preserving all debits and restoration.
// Policy56 retains a same-version lookup for one pinned immutable key in a
// closed loop. Full lookup debits remain; every write invalidates the proof.
// Policy57 fuses a selected adjacent range/String projection in a closed map
// body. Both original visits, range charges and diagnostic origins remain.
// Policy58 prepays bounded packed-map bodies and growth deltas on the same
// finite account. Generated probes/stores retain every source operation.
// Policy59 fences the SSA emission/protocol; existing finite body prices and
// source-operation caps remain in force, including slow-path synchronization.
// Policy61 lends finite parent work to isolated native children, accounts their
// shared memory concurrently, and reconciles all spending before publication.
// Policy62 includes compact-key validation and decoding inside the existing
// finite loop, growth and publication bounds, with shared worker claims intact.
// Policy63 admits both control/slot containers and their simultaneous growth;
// ABI22 pins the new control pointer and three-word payload slot layout.
// Policy64 prices the actual domain-selected slot backing and its concurrent
// growth. Full-chain body prices, source-call caps and shared claims remain.
// ABI23 binds both payload layouts; the packed Frame remains nineteen words.
// Policy65 prices complete frozen-map validation and private read probes;
// physical canonical-map import, sorting and allocation are genuinely absent.
// Policy67 admits the actual immutable-owner layout containing its inline
// shared String representation fact. Original scan charges remain in force.
// Policy68 includes the inline borrowed-offset descriptors in the actual loan
// layout. Original row/payload work and row-capacity claims stay in force.
// Policy69 admits the actual composition-state layout and requires a bounded
// chunk/table computation to fit the original prepaid join envelope. Source
// operation prices, finite allowances and later map charges are unchanged.
pub const NATIVE_PROGRAM_WORK_POLICY_VERSION: u64 = 69;
pub const NATIVE_PROGRAM_LINK_IDENTITY_BYTES: usize = 192;
const MAGIC: &[u8; 16] = b"MECHNATIVEPROG\0\0";
#[derive(Clone, Debug)]
pub struct NativeProgramObjectIdentity {
    bytes: [u8; NATIVE_PROGRAM_LINK_IDENTITY_BYTES],
    symbol: String,
}
impl NativeProgramObjectIdentity {
    pub(in crate::native_artifact) fn from_descriptor(
        d: &crate::native_artifact::program::image::NativeProgramDescriptor,
    ) -> Result<Self, &'static str> {
        let mut bytes = [0; NATIVE_PROGRAM_LINK_IDENTITY_BYTES];
        bytes[..16].copy_from_slice(MAGIC);
        let p = d.plan();
        for (at, value) in [
            (16, NATIVE_PROGRAM_TYPED_IR_VERSION),
            (24, crate::native_artifact::program::protocol::abi::VERSION),
            (32, NATIVE_PROGRAM_WORK_POLICY_VERSION),
            (112, d.stack_bytes()),
            (120, d.code_bytes()),
            (128, p.transport_input_count() as u64),
            (136, p.values().len() as u64),
            (144, p.blocks().len() as u64),
            (152, p.transport_output_count() as u64),
            (160, p.entry() as u64),
            (168, p.iterations().len() as u64),
            (176, p.constants().len() as u64),
            (
                184,
                core::mem::size_of::<crate::native_artifact::program::protocol::abi::Frame>()
                    as u64,
            ),
        ] {
            bytes[at..at + 8].copy_from_slice(&value.to_le_bytes());
        }
        bytes[40..72].copy_from_slice(p.program_revision().as_bytes());
        bytes[72..104].copy_from_slice(d.typed_ir_sha256());
        bytes[104..112].copy_from_slice(
            &crate::native_artifact::identity::host_target_tag()
                .ok_or("unsupported program target")?,
        );
        Ok(Self {
            bytes,
            symbol: format!(
                "mech_canonical_program_identity_v1_{}",
                crate::native_artifact::identity::hex(d.typed_ir_sha256())
            ),
        })
    }
    pub fn bytes(&self) -> &[u8; NATIVE_PROGRAM_LINK_IDENTITY_BYTES] {
        &self.bytes
    }
    pub fn symbol(&self) -> &str {
        &self.symbol
    }
}
pub(in crate::native_artifact) fn typed_ir_sha256(
    plan: &NativeProgramArtifactPlan,
    frontiers: &[NativeTextLineRetentionFrontier],
    joins: &[NativeSelectedTextJoin],
    lines: &[NativeTextLineMatrixRegion],
    map_owners: Option<&mech_engine::native_artifact::NativeMapOwnerWords>,
    frozen_maps: &crate::native_artifact::program::prepare::frozen_map::Layout,
    string_maps: &[mech_engine::native_artifact::NativeStringMapRegion],
) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"mech.canonical-managed-program-cfg\0");
    word(&mut hash, NATIVE_PROGRAM_TYPED_IR_VERSION); // Separate control IR version.
    word(
        &mut hash,
        crate::native_artifact::program::protocol::abi::VERSION,
    );
    word(&mut hash, NATIVE_PROGRAM_WORK_POLICY_VERSION);
    hash.update(plan.program_revision().as_bytes());
    bytes(&mut hash, b"mech.complete-frozen-map-layout.v1");
    serde_json::to_writer(ProofDigest(&mut hash), frozen_maps)
        .expect("frozen map proof contains only finite ordered integer fields");
    // These proof records contain only ordered struct/enum fields, arrays and
    // integer words (no maps, floats, pointers or Debug output). Stream their
    // complete encoding, including lengths and otherwise unused candidates,
    // rather than reconstructing selection or allocating a second image.
    bytes(&mut hash, b"mech.retained-map-candidate-layout.v1");
    struct ProofDigest<'a>(&'a mut Sha256);
    impl std::io::Write for ProofDigest<'_> {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.update(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    // The fixed proof grammar above has no fallible JSON values; the digest
    // sink is infallible. A failure still receives a distinct invalid marker.
    let encoded = serde_json::to_writer(ProofDigest(&mut hash), &map_owners);
    word(&mut hash, u64::from(encoded.is_ok()));
    word(&mut hash, frontiers.len() as u64);
    for frontier in frontiers {
        word(&mut hash, u64::from(frontier.iteration()));
        match frontier.scans_per_ordinal() {
            Some(bound) => {
                hash.update([1]);
                word(&mut hash, bound);
            }
            None => hash.update([0]),
        }
        word(
            &mut hash,
            u64::from(frontier.read().count().lines().matrix().get()),
        );
        word(&mut hash, frontier.states().len() as u64);
        for state in frontier.states() {
            word(&mut hash, u64::from(state.ordinal()));
            word(
                &mut hash,
                state.index_field().map_or(u64::MAX, |field| field as u64),
            );
            word(&mut hash, state.keep_fields().len() as u64);
            for field in state.keep_fields() {
                word(&mut hash, *field as u64);
            }
            let (first, end) = state
                .returned_range()
                .map_or((u64::MAX, u64::MAX), |(first, end)| {
                    (first as u64, end as u64)
                });
            word(&mut hash, first);
            word(&mut hash, end);
        }
    }
    word(&mut hash, plan.entry() as u64);
    word(&mut hash, plan.values().len() as u64);
    for ty in plan.values() {
        hash.update([match ty.scalar_kind() {
            None => 0,
            Some(NativeScalarKind::Bool) => 1,
            Some(NativeScalarKind::U64) => 2,
            Some(NativeScalarKind::I64) => 3,
            Some(NativeScalarKind::F64) => 4,
        }]);
        word(&mut hash, ty.schema().get() as u64);
        hash.update(ty.schema_key().as_bytes());
    }
    word(
        &mut hash,
        plan.activation_boundary().map_or(u64::MAX, u64::from),
    );
    word(&mut hash, plan.activation_nodes().len() as u64);
    for node in plan.activation_nodes() {
        word(&mut hash, u64::from(node.get()));
    }
    word(&mut hash, string_maps.len() as u64);
    for region in string_maps {
        for field in [
            region.collection(),
            region.iteration(),
            region.source().get(),
            region.map().get(),
            region.selection_site(),
        ] {
            word(&mut hash, u64::from(field));
        }
        word(&mut hash, region.owned_source_slots().len() as u64);
        for value in region.owned_source_slots() {
            word(&mut hash, u64::from(value.get()));
        }
    }
    word(&mut hash, plan.inputs().len() as u64);
    for input in plan.inputs() {
        word(&mut hash, input.input().get() as u64);
        word(&mut hash, input.slot().get() as u64);
        bytes(&mut hash, input.name().as_bytes());
        word(&mut hash, input.value().get() as u64);
    }
    // Retain complete original resource authority obligations, including unused
    // declarations. Observation/effect SSA transport is explicit compiler
    // metadata, not a synthetic application input/output or a resource opcode.
    word(&mut hash, plan.resource_requirements().len() as u64);
    for request in plan.resource_requirements() {
        for field in [
            &request.base_uri,
            &request.path,
            &request.context_name,
            &request.operation,
        ] {
            bytes(&mut hash, field.as_bytes());
        }
        hash.update([match request.intent {
            mech_core::ResourceIntent::Read => 0,
            mech_core::ResourceIntent::Send => 1,
            mech_core::ResourceIntent::Assign => 2,
        }]);
        hash.update([match request.delivery {
            mech_core::ResourceDelivery::Snapshot => 0,
            mech_core::ResourceDelivery::Live => 1,
        }]);
    }
    word(&mut hash, plan.observations().len() as u64);
    for observation in plan.observations() {
        for field in [
            observation.node().get(),
            observation.requirement().get(),
            observation.slot().get(),
            observation.ordinal(),
            observation.value().get(),
            observation.ty().schema().get(),
        ] {
            word(&mut hash, u64::from(field));
        }
        hash.update(observation.ty().schema_key().as_bytes());
    }
    word(&mut hash, plan.effects().len() as u64);
    for effect in plan.effects() {
        for field in [
            effect.node().get(),
            effect.requirement().get(),
            effect.ordinal(),
            effect.value().get(),
            effect.ty().schema().get(),
        ] {
            word(&mut hash, u64::from(field));
        }
        hash.update(effect.ty().schema_key().as_bytes());
        match effect.payload() {
            mech_engine::ArtifactSource::Slot(slot) => {
                hash.update([0]);
                word(&mut hash, u64::from(slot.get()));
            }
            mech_engine::ArtifactSource::Constant(constant) => {
                hash.update([1]);
                word(&mut hash, u64::from(constant.get()));
            }
        }
    }
    word(&mut hash, plan.transport_input_count() as u64);
    word(&mut hash, plan.transport_output_count() as u64);
    word(&mut hash, plan.constants().len() as u64);
    for constant in plan.constants() {
        word(&mut hash, constant.value().get() as u64);
        // The canonical revision commits to complete immutable constant data.
        word(&mut hash, constant.constant().schema().get() as u64);
        hash.update(constant.constant().schema_key().as_bytes());
    }

    word(&mut hash, plan.blocks().len() as u64);
    for block in plan.blocks() {
        let at = block.origin();
        word(&mut hash, at.node().get() as u64);
        for id in [at.block(), at.local()] {
            match id {
                Some(id) => {
                    hash.update([1]);
                    word(&mut hash, id as u64);
                }
                None => hash.update([0]),
            }
        }
        match block.iteration() {
            Some(id) => {
                hash.update([1]);
                word(&mut hash, id as u64);
            }
            None => hash.update([0]),
        }
        match block.action() {
            NativeIterationAction::PrepareStringSpanSelector { collection } => {
                hash.update([14]);
                word(&mut hash, u64::from(*collection));
            }
            NativeIterationAction::ProjectStringSpan { collection } => {
                hash.update([15]);
                word(&mut hash, u64::from(*collection));
            }
            NativeIterationAction::ProjectStringMapEntry { collection } => {
                hash.update([16]);
                word(&mut hash, u64::from(*collection));
            }
            NativeIterationAction::HistogramQueryStep { collection, step } => {
                hash.update([12]);
                word(&mut hash, *collection as u64);
                word(&mut hash, *step as u64);
            }
            NativeIterationAction::HistogramFinish { collection } => {
                hash.update([13]);
                word(&mut hash, *collection as u64);
            }
            NativeIterationAction::TextJoinFinish { collection } => {
                hash.update([11]);
                word(&mut hash, *collection as u64);
            }
            NativeIterationAction::PrepareU64Range { iteration } => {
                hash.update([9]);
                word(&mut hash, u64::from(*iteration));
            }
            NativeIterationAction::Structural {
                source,
                path,
                observation,
                output,
            } => {
                hash.update([8]);
                word(&mut hash, source.get() as u64);
                word(&mut hash, output.get() as u64);
                word(&mut hash, path.len() as u64);
                for step in path {
                    match step {
                        mech_engine::native_artifact::NativeStructuralStep::EnumPayload {
                            ordinal,
                        } => {
                            hash.update([0]);
                            word(&mut hash, *ordinal as u64);
                        }
                        mech_engine::native_artifact::NativeStructuralStep::TupleItem {
                            ordinal,
                        } => {
                            hash.update([1]);
                            word(&mut hash, *ordinal as u64);
                        }
                    }
                }
                match observation {
                    mech_engine::native_artifact::NativeStructuralObservation::CanonicalValue => {
                        hash.update([3]);
                    }
                    mech_engine::native_artifact::NativeStructuralObservation::U64 => {
                        hash.update([0])
                    }
                    mech_engine::native_artifact::NativeStructuralObservation::EnumOrdinal {
                        ordinal,
                    } => {
                        hash.update([1]);
                        word(&mut hash, *ordinal as u64);
                    }
                    mech_engine::native_artifact::NativeStructuralObservation::StringEqual {
                        peer,
                    } => {
                        hash.update([2]);
                        word(&mut hash, peer.get() as u64);
                    }
                }
            }
            // Commit the maintained selection as well as operand slots.
            NativeIterationAction::LibraryCall {
                template,
                operands,
                output,
            } => {
                hash.update([3]);
                bytes(&mut hash, template.primitive().name().as_bytes());
                word(&mut hash, output.get() as u64);
                word(&mut hash, operands.len() as u64);
                for v in operands {
                    word(&mut hash, v.get() as u64);
                }
                let key = template.selected_key();
                word(&mut hash, key.module_path.len() as u64);
                for part in &key.module_path {
                    bytes(&mut hash, part.as_bytes());
                }
                bytes(&mut hash, key.operation_name.as_bytes());
                use mech_core::ImplementationMemoryClass as M;
                let (tag, a, b) = match template.implementation_memory() {
                    M::NoAdditionalScratch => (0u64, 0, 0),
                    M::CloneInput { input } => (1, input, 0),
                    M::CanonicalCloneInput { input } => (2, input, 0),
                    M::AbiContiguousBridge { input, output } => (3, input, output),
                    M::ExternalMarshalling => (4, 0, 0),
                    M::MatrixSolve => (5, 0, 0),
                    M::CanonicalFinalize => (6, 0, 0),
                    M::CanonicalSortUnique => (7, 0, 0),
                    M::PackedStringMatrixOrCanonicalFinalize => (8, 0, 0),
                };
                for n in [tag, a as u64, b as u64] {
                    word(&mut hash, n);
                }
            }
            NativeIterationAction::Scalar(action) => {
                hash.update([0]);
                match action {
                    NativeMatchAction::None => hash.update([0]),
                    NativeMatchAction::Primitive {
                        op,
                        operands,
                        output,
                        ..
                    } => {
                        hash.update([1, crate::native_artifact::identity::op_tag(*op)]);
                        word(&mut hash, output.get() as u64);
                        word(&mut hash, operands.len() as u64);
                        for operand in operands {
                            word(&mut hash, operand.get() as u64);
                        }
                    }
                    NativeMatchAction::Copy { source, output } => {
                        hash.update([2]);
                        word(&mut hash, source.get() as u64);
                        word(&mut hash, output.get() as u64);
                    }
                }
            }
            NativeIterationAction::Initialize { iteration } => {
                hash.update([1]);
                word(&mut hash, *iteration as u64);
            }
            NativeIterationAction::Latch { iteration } => {
                hash.update([2]);
                word(&mut hash, *iteration as u64);
            }
            NativeIterationAction::CollectionBegin { collection } => {
                hash.update([4]);
                word(&mut hash, *collection as u64);
            }
            NativeIterationAction::CollectionElement { iteration } => {
                hash.update([5]);
                word(&mut hash, *iteration as u64);
            }
            NativeIterationAction::CollectionPush { collection, value } => {
                hash.update([6]);
                word(&mut hash, *collection as u64);
                word(&mut hash, value.get() as u64);
            }
            NativeIterationAction::CollectionFinish { collection } => {
                hash.update([7]);
                word(&mut hash, *collection as u64);
            }
        }
        match block.terminator() {
            NativeIterationTerminator::TailHeader { iteration, body } => {
                hash.update([2]);
                word(&mut hash, iteration as u64);
                word(&mut hash, body as u64);
            }
            NativeIterationTerminator::Scalar(terminator) => {
                hash.update([0]);
                match terminator {
                    NativeMatchTerminator::Jump(next) => {
                        hash.update([1]);
                        word(&mut hash, next as u64);
                    }
                    NativeMatchTerminator::Branch { condition, yes, no } => {
                        hash.update([2]);
                        word(&mut hash, condition.get() as u64);
                        word(&mut hash, yes as u64);
                        word(&mut hash, no as u64);
                    }
                    NativeMatchTerminator::LiteralBranch {
                        left,
                        right,
                        yes,
                        no,
                    } => {
                        hash.update([3]);
                        for value in [left.get(), right.get(), yes, no] {
                            word(&mut hash, value as u64);
                        }
                    }
                    NativeMatchTerminator::Complete => hash.update([4]),
                    NativeMatchTerminator::Reject => hash.update([5]),
                }
            }
            NativeIterationTerminator::Header {
                iteration,
                body,
                exit,
            } => {
                hash.update([1]);
                for id in [iteration, body, exit] {
                    word(&mut hash, id as u64);
                }
            }
        }
    }
    word(&mut hash, plan.value_scopes().len() as u64);
    for scope in plan.value_scopes() {
        match scope {
            NativeIterationValueScope::Immutable { iteration } => {
                hash.update([0]);
                match iteration {
                    Some(id) => {
                        hash.update([1]);
                        word(&mut hash, *id as u64);
                    }
                    None => hash.update([0]),
                }
            }
            NativeIterationValueScope::Carrier { iteration } => {
                hash.update([1]);
                word(&mut hash, *iteration as u64);
            }
        }
    }
    word(&mut hash, plan.iterations().len() as u64);
    for iteration in plan.iterations() {
        let iteration = iteration.as_ref();
        word(&mut hash, iteration.origin().node().get() as u64);
        for id in [
            iteration.origin().block(),
            iteration.origin().local(),
            iteration.parent(),
        ] {
            match id {
                Some(id) => {
                    hash.update([1]);
                    word(&mut hash, id as u64);
                }
                None => hash.update([0]),
            }
        }
        match iteration {
            mech_engine::native_artifact::NativeControlLoopRef::Iteration(fold) => {
                hash.update([0]);
                word(&mut hash, fold.count());
                for value in [fold.seed(), fold.carrier(), fold.next(), fold.result()] {
                    word(&mut hash, value.get() as u64);
                }
            }
            mech_engine::native_artifact::NativeControlLoopRef::Collection(collection) => {
                hash.update([1]);
                word(&mut hash, collection.collection() as u64);
                match collection.source() {
                    mech_engine::native_artifact::NativeCollectionSource::Value(value) => {
                        hash.update([0]);
                        word(&mut hash, u64::from(value.get()));
                    }
                    mech_engine::native_artifact::NativeCollectionSource::U64Range(cursor) => {
                        hash.update([1]);
                        word(&mut hash, u64::from(cursor.prepare()));
                        word(&mut hash, u64::from(cursor.semantic_output().get()));
                        word(&mut hash, u64::from(cursor.before_collection_begin()));
                        for value in cursor.operands() {
                            word(&mut hash, u64::from(value.get()));
                        }
                        let template = cursor.template();
                        hash.update(template.program_revision().as_bytes());
                        bytes(&mut hash, template.primitive().name().as_bytes());
                        let at = template.origin();
                        word(&mut hash, u64::from(at.node().get()));
                        for part in [at.block(), at.local()] {
                            word(&mut hash, part.map_or(u64::MAX, u64::from));
                        }
                        word(&mut hash, template.inputs().len() as u64);
                        for schema in template.inputs() {
                            hash.update(schema.key().as_bytes());
                        }
                        hash.update(template.output().key().as_bytes());
                        let selected = template.selected_key();
                        word(&mut hash, selected.module_path.len() as u64);
                        for part in &selected.module_path {
                            bytes(&mut hash, part.as_bytes());
                        }
                        bytes(&mut hash, selected.operation_name.as_bytes());
                        // The only eligible maintained factories have this
                        // exact class; the engine proof rejects all others.
                        word(
                            &mut hash,
                            u64::from(
                                template.implementation_memory()
                                    == mech_core::ImplementationMemoryClass::NoAdditionalScratch,
                            ),
                        );
                    }
                }
                match collection.element() {
                    Some(value) => {
                        hash.update([1]);
                        word(&mut hash, value.get() as u64);
                    }
                    None => hash.update([0]),
                }
            }
            mech_engine::native_artifact::NativeControlLoopRef::Tail(tail) => {
                hash.update([2]);
                for value in [
                    tail.seed(),
                    tail.carrier(),
                    tail.next(),
                    tail.returned(),
                    tail.result(),
                ] {
                    word(&mut hash, value.get() as u64);
                }
                word(&mut hash, tail.finish() as u64);
            }
        }
        for id in [
            iteration.initialize(),
            iteration.header(),
            iteration.body(),
            iteration.latch(),
            iteration.exit(),
        ] {
            word(&mut hash, id as u64);
        }
    }

    word(&mut hash, plan.collections().len() as u64);
    for collection in plan.collections() {
        word(&mut hash, collection.origin().node().get() as u64);
        for id in [
            collection.origin().block(),
            collection.origin().local(),
            collection.parent(),
            collection.parent_collection(),
        ] {
            match id {
                Some(id) => {
                    hash.update([1]);
                    word(&mut hash, id as u64);
                }
                None => hash.update([0]),
            }
        }
        match collection.kind() {
            mech_engine::ComprehensionKind::Matrix => hash.update([0]),
            mech_engine::ComprehensionKind::MatrixPreserveShape => hash.update([1]),
            mech_engine::ComprehensionKind::Set => hash.update([2]),
            mech_engine::ComprehensionKind::MatrixHorizontalBlocks { rows } => {
                hash.update([3]);
                match rows {
                    mech_engine::MatrixBlockRows::InferOrZero => hash.update([0]),
                    mech_engine::MatrixBlockRows::Fixed(n) => {
                        hash.update([1]);
                        word(&mut hash, n);
                    }
                    mech_engine::MatrixBlockRows::InputMatrixElements { input } => {
                        hash.update([2]);
                        word(&mut hash, input as u64);
                    }
                }
            }
        }
        for value in [
            collection.output().get(),
            collection.begin(),
            collection.push(),
            collection.finish(),
        ] {
            word(&mut hash, value as u64);
        }
        match collection.result() {
            mech_engine::native_artifact::NativeCollectionResult::Materialized => hash.update([0]),
            mech_engine::native_artifact::NativeCollectionResult::GuardedHistogramQuery(sink) => {
                use mech_engine::native_artifact::{
                    NativeHistogramQueryPhase as P, NativeHistogramQuerySetShape as S,
                    NativeHistogramQueryStepKind as K,
                };
                hash.update([2]);
                for value in [sink.query(), sink.match_result(), sink.zero()] {
                    word(&mut hash, value.get() as u64);
                }
                hash.update([match sink.set_shape() {
                    S::Dynamic => 0,
                    S::FreeExact => 1,
                }]);
                for site in [sink.true_copy(), sink.false_copy()] {
                    word(&mut hash, site as u64);
                }
                word(&mut hash, sink.bool_tests().len() as u64);
                for &site in sink.bool_tests() {
                    word(&mut hash, site as u64);
                }
                word(&mut hash, sink.steps().len() as u64);
                for step in sink.steps() {
                    word(&mut hash, step.site() as u64);
                    let (tag, phase) = match step.kind() {
                        K::Histogram => (0, 0),
                        K::Keys => (1, 0),
                        K::Set => (2, 0),
                        K::Membership => (3, 0),
                        K::Lookup => (4, 0),
                        K::Cleanup => (5, 0),
                        K::Alias { phase } => (
                            6,
                            match phase {
                                P::ProducerReady => 0,
                                P::HistogramReady => 1,
                                P::KeysReady => 2,
                                P::SetReady => 3,
                                P::Guarded => 4,
                                P::LookupRead => 5,
                            },
                        ),
                    };
                    hash.update([tag, phase]);
                    word(&mut hash, step.operands().len() as u64);
                    for value in step.operands() {
                        word(&mut hash, value.get() as u64);
                    }
                    match step.output() {
                        Some(value) => {
                            hash.update([1]);
                            word(&mut hash, value.get() as u64);
                        }
                        None => hash.update([0]),
                    }
                    match step.template() {
                        Some(template) => {
                            hash.update([1]);
                            selected_template(&mut hash, template);
                        }
                        None => hash.update([0]),
                    }
                }
            }
            mech_engine::native_artifact::NativeCollectionResult::Histogram(sink) => {
                hash.update([3]);
                word(&mut hash, sink.consumer() as u64);
                word(&mut hash, sink.output().get() as u64);
                selected_template(&mut hash, sink.template());
            }
            mech_engine::native_artifact::NativeCollectionResult::TextJoin(sink) => {
                hash.update([1]);
                word(&mut hash, sink.consumer() as u64);
                word(&mut hash, sink.separator().get() as u64);
                word(&mut hash, sink.output().get() as u64);
                let template = sink.template();
                word(&mut hash, template.origin().node().get() as u64);
                word(
                    &mut hash,
                    template.origin().block().map_or(u64::MAX, u64::from),
                );
                word(
                    &mut hash,
                    template.origin().local().map_or(u64::MAX, u64::from),
                );
                hash.update(template.program_revision().as_bytes());
                bytes(&mut hash, template.primitive().name().as_bytes());
                let key = template.selected_key();
                word(&mut hash, key.module_path.len() as u64);
                for part in &key.module_path {
                    bytes(&mut hash, part.as_bytes());
                }
                bytes(&mut hash, key.operation_name.as_bytes());
                word(&mut hash, template.inputs().len() as u64);
                for input in template.inputs() {
                    bytes(&mut hash, &input.canonical_bytes());
                }
                bytes(&mut hash, &template.output().canonical_bytes());
                match template.contract().canonical_bytes() {
                    Ok(encoded) => {
                        hash.update([1]);
                        bytes(&mut hash, &encoded);
                    }
                    Err(_) => hash.update([0]), // invalid private plans are rejected before entry
                }
                hash.update([6]); // exact CanonicalFinalize witness, checked by proof
            }
        }
        match collection.string_span_projection() {
            Some(projection) => {
                hash.update([1]);
                for site in [projection.range_site(), projection.selection_site()] {
                    word(&mut hash, u64::from(site));
                }
                for value in [
                    projection.operands()[0],
                    projection.operands()[1],
                    projection.source(),
                    projection.selector(),
                    projection.output(),
                ] {
                    word(&mut hash, u64::from(value.get()));
                }
                selected_template(&mut hash, projection.range_template());
                selected_template(&mut hash, projection.selection_template());
            }
            None => hash.update([0]),
        }
        match collection.string_element_map_projection() {
            Some(projection) => {
                hash.update([1]);
                for site in [
                    projection.iteration(),
                    projection.element_site(),
                    projection.selection_site(),
                ] {
                    word(&mut hash, u64::from(site));
                }
                for value in [
                    projection.source(),
                    projection.map(),
                    projection.element(),
                    projection.output(),
                ] {
                    word(&mut hash, u64::from(value.get()));
                }
                selected_template(&mut hash, projection.selection_template());
            }
            None => hash.update([0]),
        }
        word(&mut hash, collection.generators().len() as u64);
        for &generator in collection.generators() {
            word(&mut hash, generator as u64);
        }
    }

    word(&mut hash, plan.outputs().len() as u64);
    for output in plan.outputs() {
        word(&mut hash, output.output().get() as u64);
        word(&mut hash, output.slot().get() as u64);
        bytes(&mut hash, output.name().as_bytes());
        word(&mut hash, output.value().get() as u64);
    }
    word(&mut hash, joins.len() as u64);
    for join in joins {
        word(&mut hash, u64::from(join.selection()));
        word(&mut hash, u64::from(join.consumer()));
        for value in [
            join.source(),
            join.selector(),
            join.selected(),
            join.separator(),
            join.output(),
        ] {
            word(&mut hash, u64::from(value.get()));
        }
        match join.range() {
            Some(range) => {
                hash.update([1]);
                word(&mut hash, u64::from(range.producer()));
                word(&mut hash, u64::from(range.start().get()));
                word(&mut hash, u64::from(range.end().get()));
                hash.update([u8::from(range.inclusive())]);
            }
            None => hash.update([0]),
        }
    }
    word(&mut hash, lines.len() as u64);
    for line in lines {
        word(&mut hash, u64::from(line.producer()));
        word(&mut hash, u64::from(line.source().get()));
        word(&mut hash, u64::from(line.matrix().get()));
        for sites in [line.lengths(), line.borrowed_regions(), line.selections()] {
            word(&mut hash, sites.len() as u64);
            for &site in sites {
                word(&mut hash, u64::from(site));
            }
        }
    }
    hash.finalize().into()
}
