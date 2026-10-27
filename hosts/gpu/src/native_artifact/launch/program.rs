//! Process-lifetime bindings to independently checked build-owned images.
use crate::native_artifact::{
    LinkedProgramArtifact, NATIVE_PROGRAM_LINK_IDENTITY_BYTES, NativeLinkError,
};
use mech_core::FunctionCatalog;
use std::ffi::c_void;
type Entry = unsafe extern "C" fn(*mut c_void) -> u64;
type Catalog = fn() -> Result<FunctionCatalog, String>;
pub(in crate::native_artifact::launch) struct ProgramCode {
    image: &'static [u8],
    identity: &'static [u8; NATIVE_PROGRAM_LINK_IDENTITY_BYTES],
    entry: Entry,
    pub(in crate::native_artifact::launch) catalog: Catalog,
}
impl ProgramCode {
    pub(in crate::native_artifact::launch) fn bind(
        &self,
        catalog: &FunctionCatalog,
    ) -> Result<LinkedProgramArtifact, NativeLinkError> {
        // SAFETY: only the unsafe same-Object binding constructors create this
        // private code owner. The loader and complete identity checks still run.
        unsafe {
            LinkedProgramArtifact::from_build_owned_image(
                self.image,
                catalog,
                self.identity,
                self.entry,
            )
        }
    }
}
/// A checked-build binding for the explicit managed qualification shell.
pub struct BuildOwnedProgramBinding(pub(in crate::native_artifact::launch) ProgramCode);
impl BuildOwnedProgramBinding {
    /// # Safety
    /// Image, identity and entry must be process-lifetime data from the SAME
    /// unchanged checked emitter Object/build. The build must verify every
    /// source/bytecode/Object/image/generated-input pin. Entry obeys the
    /// synchronous admitted private ABI; matching bytes alone grant no trust.
    pub unsafe fn from_build_owned_image(
        image: &'static [u8],
        identity: &'static [u8; NATIVE_PROGRAM_LINK_IDENTITY_BYTES],
        entry: Entry,
        catalog: Catalog,
    ) -> Self {
        Self(ProgramCode {
            image,
            identity,
            entry,
            catalog,
        })
    }
}
#[cfg(feature = "canonical-native-program-host-runtime")]
/// A same-build program/resource image binding for the capability-secured shell.
pub struct BuildOwnedHostedBinding {
    pub(in crate::native_artifact::launch) code: ProgramCode,
    pub(in crate::native_artifact::launch) resource_image: &'static [u8],
    pub(in crate::native_artifact::launch) route: &'static str,
}
#[cfg(feature = "canonical-native-program-host-runtime")]
impl BuildOwnedHostedBinding {
    /// # Safety
    /// All image/entry requirements of BuildOwnedProgramBinding apply. The
    /// resource image must be the unchanged image of that SAME source/bytecode
    /// revision, pinned by the checked build. It grants no runtime authority;
    /// actual factories, schemas, resource admission and identity are rechecked.
    pub unsafe fn from_build_owned_image(
        image: &'static [u8],
        resources: &'static [u8],
        identity: &'static [u8; NATIVE_PROGRAM_LINK_IDENTITY_BYTES],
        entry: Entry,
        catalog: Catalog,
        route: &'static str,
    ) -> Self {
        Self {
            code: ProgramCode {
                image,
                identity,
                entry,
                catalog,
            },
            resource_image: resources,
            route,
        }
    }
}
