//! Process-lifetime bindings to independently checked build-owned images.
use crate::native_artifact::{
    LinkedProgramArtifact, NATIVE_PROGRAM_IMAGE_BINDING_BYTES, NATIVE_PROGRAM_LINK_IDENTITY_BYTES,
    NativeLinkError,
};
use mech_core::FunctionCatalog;
use std::ffi::c_void;
type Entry = unsafe extern "C" fn(*mut c_void) -> u64;
type Catalog = fn() -> Result<FunctionCatalog, String>;
pub(in crate::native_artifact::launch) struct ProgramCode {
    image: ProgramImage,
    pub(in crate::native_artifact::launch) catalog: Catalog,
}
#[derive(Clone, Copy)]
enum ProgramImage {
    Compiled(&'static crate::native_artifact::NativeProgramModule),
    Bound {
        image: &'static [u8],
        identity: &'static [u8; NATIVE_PROGRAM_LINK_IDENTITY_BYTES],
        image_binding: Option<&'static [u8; NATIVE_PROGRAM_IMAGE_BINDING_BYTES]>,
        entry: Entry,
    },
}
impl ProgramCode {
    pub(in crate::native_artifact::launch) fn bind(
        &self,
        catalog: &FunctionCatalog,
    ) -> Result<LinkedProgramArtifact, NativeLinkError> {
        // SAFETY: only the unsafe same-Object binding constructors create this
        // private code owner. The loader and complete identity checks still run.
        unsafe {
            let ProgramImage::Bound {
                image,
                identity,
                image_binding,
                entry,
            } = self.image
            else {
                let ProgramImage::Compiled(module) = self.image else {
                    unreachable!()
                };
                return LinkedProgramArtifact::from_compiled_module(module, catalog);
            };
            if let Some(binding) = image_binding {
                return LinkedProgramArtifact::from_object_bound_image(
                    image, catalog, identity, binding, entry,
                );
            }
            LinkedProgramArtifact::from_build_owned_image(image, catalog, identity, entry)
        }
    }
}
/// A checked-build binding for the explicit managed qualification shell.
pub struct BuildOwnedProgramBinding(pub(in crate::native_artifact::launch) ProgramCode);
impl BuildOwnedProgramBinding {
    /// # Safety
    /// All LinkedProgramArtifact::from_compiled_module provenance requirements
    /// apply. Only the genuine immutable compiler module may be supplied.
    pub unsafe fn from_compiled_module(
        module: &'static crate::native_artifact::NativeProgramModule,
        catalog: Catalog,
    ) -> Self {
        Self(ProgramCode {
            image: ProgramImage::Compiled(module),
            catalog,
        })
    }

    /// # Safety
    /// All from_build_owned_image requirements apply. image_binding must also
    /// be the genuine read-only descriptor-binding symbol from that SAME Object.
    /// Matching caller-supplied digests cannot satisfy checked-code provenance.
    pub unsafe fn from_object_bound_image(
        image: &'static [u8],
        identity: &'static [u8; NATIVE_PROGRAM_LINK_IDENTITY_BYTES],
        image_binding: &'static [u8; NATIVE_PROGRAM_IMAGE_BINDING_BYTES],
        entry: Entry,
        catalog: Catalog,
    ) -> Self {
        Self(ProgramCode {
            image: ProgramImage::Bound {
                image,
                identity,
                image_binding: Some(image_binding),
                entry,
            },
            catalog,
        })
    }
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
            image: ProgramImage::Bound {
                image,
                identity,
                image_binding: None,
                entry,
            },
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
    /// All LinkedProgramArtifact::from_compiled_module requirements apply. The
    /// resource image must belong to the same pinned build; its independent
    /// resource/capability admission and provider checks remain mandatory.
    pub unsafe fn from_compiled_module(
        module: &'static crate::native_artifact::NativeProgramModule,
        resources: &'static [u8],
        catalog: Catalog,
        route: &'static str,
    ) -> Self {
        Self {
            code: ProgramCode {
                image: ProgramImage::Compiled(module),
                catalog,
            },
            resource_image: resources,
            route,
        }
    }

    /// # Safety
    /// All from_build_owned_image requirements apply. image_binding is the
    /// genuine read-only binding from the SAME checked program Object. The
    /// resource image still undergoes its ordinary independent admission checks.
    pub unsafe fn from_object_bound_image(
        image: &'static [u8],
        resources: &'static [u8],
        identity: &'static [u8; NATIVE_PROGRAM_LINK_IDENTITY_BYTES],
        image_binding: &'static [u8; NATIVE_PROGRAM_IMAGE_BINDING_BYTES],
        entry: Entry,
        catalog: Catalog,
        route: &'static str,
    ) -> Self {
        Self {
            code: ProgramCode {
                image: ProgramImage::Bound {
                    image,
                    identity,
                    image_binding: Some(image_binding),
                    entry,
                },
                catalog,
            },
            resource_image: resources,
            route,
        }
    }
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
                image: ProgramImage::Bound {
                    image,
                    identity,
                    image_binding: None,
                    entry,
                },
                catalog,
            },
            resource_image: resources,
            route,
        }
    }
}
