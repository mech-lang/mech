//! Explicit services supplied while Mech functions execute.

use crate::{LegacyValue, MResult, MechError, MechErrorKind, ValRef};

#[cfg(feature = "no_std")]
use alloc::{format, string::String};
#[cfg(not(feature = "no_std"))]
use std::string::String;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum ResourceIntent {
    Read = 1,
    Assign = 2,
    Send = 3,
}

impl ResourceIntent {
    pub(crate) fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Read),
            2 => Some(Self::Assign),
            3 => Some(Self::Send),
            _ => None,
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum ResourceDelivery {
    Snapshot = 0,
    Live = 1,
}

impl ResourceDelivery {
    pub(crate) fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Snapshot),
            1 => Some(Self::Live),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExecutionHostFunctionRequest {
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExecutionResourceRequest {
    pub base_uri: String,
    pub path: String,
    pub context_name: String,
    pub operation: String,
    pub intent: ResourceIntent,
    pub delivery: ResourceDelivery,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ApplicationRequirement {
    HostFunction(ExecutionHostFunctionRequest),
    Resource(ExecutionResourceRequest),
}

pub trait MechExecutionServices {
    fn invoke_host_function(
        &mut self,
        request: &ExecutionHostFunctionRequest,
        arguments: &[LegacyValue],
    ) -> MResult<LegacyValue>;

    fn read_resource(&mut self, request: &ExecutionResourceRequest) -> MResult<LegacyValue>;

    fn write_resource(
        &mut self,
        request: &ExecutionResourceRequest,
        value: &LegacyValue,
    ) -> MResult<()>;

    /// Retains a live delivery target. Repeating the same interpreter, request,
    /// and target binding must be idempotent.
    fn bind_live_resource(
        &mut self,
        interpreter_id: u64,
        request: &ExecutionResourceRequest,
        target: ValRef,
    ) -> MResult<()>;
}

#[derive(Debug, Default)]
pub struct NoMechExecutionServices;

impl MechExecutionServices for NoMechExecutionServices {
    fn invoke_host_function(
        &mut self,
        request: &ExecutionHostFunctionRequest,
        _arguments: &[LegacyValue],
    ) -> MResult<LegacyValue> {
        Err(MechError::new(
            HostFunctionExecutionUnsupported {
                request: request.clone(),
            },
            None,
        ))
    }

    fn read_resource(&mut self, request: &ExecutionResourceRequest) -> MResult<LegacyValue> {
        Err(MechError::new(
            ResourceReadExecutionUnsupported {
                request: request.clone(),
            },
            None,
        ))
    }

    fn write_resource(
        &mut self,
        request: &ExecutionResourceRequest,
        _value: &LegacyValue,
    ) -> MResult<()> {
        Err(MechError::new(
            ResourceWriteExecutionUnsupported {
                request: request.clone(),
            },
            None,
        ))
    }

    fn bind_live_resource(
        &mut self,
        interpreter_id: u64,
        request: &ExecutionResourceRequest,
        _target: ValRef,
    ) -> MResult<()> {
        Err(MechError::new(
            LiveResourceBindingUnsupported {
                interpreter_id,
                request: request.clone(),
            },
            None,
        ))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostFunctionExecutionUnsupported {
    pub request: ExecutionHostFunctionRequest,
}

impl MechErrorKind for HostFunctionExecutionUnsupported {
    fn name(&self) -> &str {
        "HostFunctionExecutionUnsupported"
    }

    fn message(&self) -> String {
        format!(
            "host-function request {:?} requires execution services",
            self.request,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceReadExecutionUnsupported {
    pub request: ExecutionResourceRequest,
}

impl MechErrorKind for ResourceReadExecutionUnsupported {
    fn name(&self) -> &str {
        "ResourceReadExecutionUnsupported"
    }

    fn message(&self) -> String {
        format!(
            "resource-read request {:?} requires execution services",
            self.request,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceWriteExecutionUnsupported {
    pub request: ExecutionResourceRequest,
}

impl MechErrorKind for ResourceWriteExecutionUnsupported {
    fn name(&self) -> &str {
        "ResourceWriteExecutionUnsupported"
    }

    fn message(&self) -> String {
        format!(
            "resource-write request {:?} requires execution services",
            self.request,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveResourceBindingUnsupported {
    pub interpreter_id: u64,
    pub request: ExecutionResourceRequest,
}

impl MechErrorKind for LiveResourceBindingUnsupported {
    fn name(&self) -> &str {
        "LiveResourceBindingUnsupported"
    }

    fn message(&self) -> String {
        format!(
            "live-resource binding request {:?} for interpreter {} requires execution services",
            self.request, self.interpreter_id,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Ref;

    fn resource_request() -> ExecutionResourceRequest {
        ExecutionResourceRequest {
            base_uri: "test://resource".into(),
            path: "items/0".into(),
            context_name: "input".into(),
            operation: "read".into(),
            intent: ResourceIntent::Read,
            delivery: ResourceDelivery::Live,
        }
    }

    #[test]
    fn no_services_reports_structured_errors_with_complete_requests() {
        let mut services = NoMechExecutionServices;
        let host_request = ExecutionHostFunctionRequest {
            name: "test/host".into(),
        };
        let resource_request = resource_request();

        let host_error = services
            .invoke_host_function(&host_request, &[])
            .unwrap_err();
        assert_eq!(host_error.kind_name(), "HostFunctionExecutionUnsupported");
        assert_eq!(
            host_error
                .kind_as::<HostFunctionExecutionUnsupported>()
                .unwrap()
                .request,
            host_request,
        );

        let read_error = services.read_resource(&resource_request).unwrap_err();
        assert_eq!(read_error.kind_name(), "ResourceReadExecutionUnsupported");
        assert_eq!(
            read_error
                .kind_as::<ResourceReadExecutionUnsupported>()
                .unwrap()
                .request,
            resource_request,
        );

        let write_error = services
            .write_resource(&resource_request, &LegacyValue::Empty)
            .unwrap_err();
        assert_eq!(write_error.kind_name(), "ResourceWriteExecutionUnsupported");
        assert_eq!(
            write_error
                .kind_as::<ResourceWriteExecutionUnsupported>()
                .unwrap()
                .request,
            resource_request,
        );

        let bind_error = services
            .bind_live_resource(17, &resource_request, Ref::new(LegacyValue::Empty))
            .unwrap_err();
        assert_eq!(bind_error.kind_name(), "LiveResourceBindingUnsupported");
        let binding = bind_error
            .kind_as::<LiveResourceBindingUnsupported>()
            .unwrap();
        assert_eq!(binding.interpreter_id, 17);
        assert_eq!(binding.request, resource_request);
    }
}
