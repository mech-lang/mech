#[cfg(feature = "semantic-compiler")]
use mech_core::ChangeDetectionPolicy;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EkfKernel {
    TrigonometricState,
    MotionJacobian,
    ControlJacobian,
    PredictedState,
    PredictedCovariance,
    LandmarkDeltaAndRange,
    PredictedMeasurement,
    MeasurementJacobian,
    InnovationCovariance,
    Solve2x2,
    KalmanGain,
    Innovation,
    CorrectedState,
    JosephCovarianceUpdate,
    CovarianceSymmetrization,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EkfPredicate {
    CandidateFinite,
    CovariancePositiveDiagonal,
    CovarianceSymmetric,
}

#[cfg(feature = "semantic-compiler")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FrozenEkfOperation {
    Kernel(EkfKernel),
    Predicate(EkfPredicate),
}

#[cfg(feature = "semantic-compiler")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FrozenEkfValueShape {
    F64,
    Bool,
    Vector(usize),
    Matrix { rows: usize, columns: usize },
}

#[cfg(feature = "semantic-compiler")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FrozenEkfOperationSpec {
    pub operation: FrozenEkfOperation,
    pub canonical_name: &'static str,
    pub module_item: &'static str,
    pub inputs: &'static [FrozenEkfValueShape],
    pub output: FrozenEkfValueShape,
    pub change_detection: ChangeDetectionPolicy,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct EkfConstants {
    pub dt: f64,
    pub landmark: [f64; 2],
    pub process_covariance: [f64; 4],
    pub measurement_covariance: [f64; 4],
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct EkfScratch {
    pub trig: [f64; 2],
    pub motion_jacobian: [f64; 9],
    pub control_jacobian: [f64; 6],
    pub predicted_state: [f64; 3],
    pub predicted_covariance: [f64; 9],
    pub delta_range: [f64; 3],
    pub predicted_measurement: [f64; 2],
    pub measurement_jacobian: [f64; 6],
    pub innovation_covariance: [f64; 4],
    pub inverse_innovation: [f64; 4],
    pub gain: [f64; 6],
    pub innovation: [f64; 2],
    pub corrected_state: [f64; 3],
    pub corrected_covariance: [f64; 9],
    pub symmetrized_covariance: [f64; 9],
}

#[cfg(feature = "semantic-compiler")]
use ChangeDetectionPolicy::{ExactScalar, KernelReported};
#[cfg(feature = "semantic-compiler")]
use EkfKernel::*;
#[cfg(feature = "semantic-compiler")]
use EkfPredicate::*;
#[cfg(feature = "semantic-compiler")]
use FrozenEkfOperation::{Kernel, Predicate};
#[cfg(feature = "semantic-compiler")]
use FrozenEkfValueShape::{Bool, F64, Matrix, Vector};

#[cfg(feature = "semantic-compiler")]
const V2: FrozenEkfValueShape = Vector(2);
#[cfg(feature = "semantic-compiler")]
const V3: FrozenEkfValueShape = Vector(3);
#[cfg(feature = "semantic-compiler")]
const V4: FrozenEkfValueShape = Vector(4);
#[cfg(feature = "semantic-compiler")]
const M2: FrozenEkfValueShape = Matrix {
    rows: 2,
    columns: 2,
};
#[cfg(feature = "semantic-compiler")]
const M3: FrozenEkfValueShape = Matrix {
    rows: 3,
    columns: 3,
};
#[cfg(feature = "semantic-compiler")]
const M2X3: FrozenEkfValueShape = Matrix {
    rows: 2,
    columns: 3,
};
#[cfg(feature = "semantic-compiler")]
const M3X2: FrozenEkfValueShape = Matrix {
    rows: 3,
    columns: 2,
};

#[cfg(feature = "semantic-compiler")]
macro_rules! spec {
    ($operation:expr, $item:literal, $inputs:expr, $output:expr, $change:expr $(,)?) => {
        FrozenEkfOperationSpec {
            operation: $operation,
            canonical_name: concat!("ekf/", $item),
            module_item: $item,
            inputs: $inputs,
            output: $output,
            change_detection: $change,
        }
    };
}

#[cfg(feature = "semantic-compiler")]
pub(crate) const FROZEN_EKF_OPERATIONS: [FrozenEkfOperationSpec; 18] = [
    spec!(
        Kernel(TrigonometricState),
        "trigonometric-state",
        &[V3],
        V2,
        KernelReported,
    ),
    spec!(
        Kernel(MotionJacobian),
        "motion-jacobian",
        &[V3, V4, V2, F64],
        M3,
        KernelReported,
    ),
    spec!(
        Kernel(ControlJacobian),
        "control-jacobian",
        &[V2, F64],
        M3X2,
        KernelReported,
    ),
    spec!(
        Kernel(PredictedState),
        "predicted-state",
        &[V3, V4, V2, F64],
        V3,
        KernelReported,
    ),
    spec!(
        Kernel(PredictedCovariance),
        "predicted-covariance",
        &[M3, M3, M3X2, M2],
        M3,
        KernelReported,
    ),
    spec!(
        Kernel(LandmarkDeltaAndRange),
        "landmark-delta-and-range",
        &[V3, V2],
        V3,
        KernelReported,
    ),
    spec!(
        Kernel(PredictedMeasurement),
        "predicted-measurement",
        &[V3, V3],
        V2,
        KernelReported,
    ),
    spec!(
        Kernel(MeasurementJacobian),
        "measurement-jacobian",
        &[V3],
        M2X3,
        KernelReported,
    ),
    spec!(
        Kernel(InnovationCovariance),
        "innovation-covariance",
        &[M3, M2X3, M2],
        M2,
        KernelReported,
    ),
    spec!(Kernel(Solve2x2), "solve-2x2", &[M2], M2, KernelReported),
    spec!(
        Kernel(KalmanGain),
        "kalman-gain",
        &[M3, M2X3, M2],
        M3X2,
        KernelReported,
    ),
    spec!(
        Kernel(Innovation),
        "innovation",
        &[V4, V2],
        V2,
        KernelReported,
    ),
    spec!(
        Kernel(CorrectedState),
        "corrected-state",
        &[V3, M3X2, V2],
        V3,
        KernelReported,
    ),
    spec!(
        Kernel(JosephCovarianceUpdate),
        "joseph-covariance-update",
        &[M3, M2X3, M3X2, M2],
        M3,
        KernelReported,
    ),
    spec!(
        Kernel(CovarianceSymmetrization),
        "covariance-symmetrization",
        &[M3],
        M3,
        KernelReported,
    ),
    spec!(
        Predicate(CandidateFinite),
        "candidate-finite",
        &[V3, M3],
        Bool,
        ExactScalar,
    ),
    spec!(
        Predicate(CovariancePositiveDiagonal),
        "covariance-positive-diagonal",
        &[M3],
        Bool,
        ExactScalar,
    ),
    spec!(
        Predicate(CovarianceSymmetric),
        "covariance-symmetric",
        &[M3],
        Bool,
        ExactScalar,
    ),
];

#[cfg(feature = "semantic-compiler")]
pub(crate) fn operation_spec(operation: FrozenEkfOperation) -> &'static FrozenEkfOperationSpec {
    FROZEN_EKF_OPERATIONS
        .iter()
        .find(|spec| spec.operation == operation)
        .expect("every frozen EKF operation has exactly one specification")
}
