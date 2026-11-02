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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FrozenEkfOperation {
    Kernel(EkfKernel),
    Predicate(EkfPredicate),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FrozenEkfValueShape {
    F64,
    Bool,
    Vector(usize),
    Matrix { rows: usize, columns: usize },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FrozenEkfOperationSpec {
    pub operation: FrozenEkfOperation,
    pub canonical_name: &'static str,
    pub module_item: &'static str,
    pub inputs: &'static [FrozenEkfValueShape],
    pub output: FrozenEkfValueShape,
    pub change_detection: ChangeDetectionPolicy,
}

pub(crate) fn closed_operation(name: &str) -> Option<&'static FrozenEkfOperationSpec> {
    FROZEN_EKF_OPERATIONS
        .iter()
        .find(|spec| spec.module_item == name)
}

pub(crate) enum ClosedEkfResult {
    Numbers { values: [f64; 9], count: usize },
    Bool(bool),
}

/// Allocation-free adapter to the exact mathematics used by resident kernels.
/// Callers validate the maintained arity/layout and admit fixed buffers first.
pub(crate) fn evaluate_closed(
    operation: FrozenEkfOperation,
    inputs: &[[f64; 9]; 4],
) -> Result<ClosedEkfResult, super::math::EkfMathError> {
    use super::math;
    fn array<const N: usize>(input: &[f64; 9]) -> [f64; N] {
        core::array::from_fn(|index| input[index])
    }
    fn numbers<const N: usize>(input: [f64; N]) -> ClosedEkfResult {
        let mut values = [0.0; 9];
        values[..N].copy_from_slice(&input);
        ClosedEkfResult::Numbers { values, count: N }
    }
    let [a, b, c, d] = inputs;
    Ok(match operation {
        Kernel(TrigonometricState) => numbers(math::trigonometric_state(&array(a))),
        Kernel(MotionJacobian) => numbers(math::motion_jacobian(&array(b), &array(c), d[0])),
        Kernel(ControlJacobian) => numbers(math::control_jacobian(&array(a), b[0])),
        Kernel(PredictedState) => {
            numbers(math::predicted_state(&array(a), &array(b), &array(c), d[0]))
        }
        Kernel(PredictedCovariance) => {
            numbers(math::predicted_covariance(a, b, &array(c), &array(d)))
        }
        Kernel(LandmarkDeltaAndRange) => {
            numbers(math::landmark_delta_and_range(&array(a), &array(b))?)
        }
        Kernel(PredictedMeasurement) => numbers(math::predicted_measurement(&array(a), &array(b))),
        Kernel(MeasurementJacobian) => numbers(math::measurement_jacobian(&array(a))),
        Kernel(InnovationCovariance) => {
            numbers(math::innovation_covariance(a, &array(b), &array(c)))
        }
        Kernel(Solve2x2) => numbers(math::solve_2x2(&array(a))?),
        Kernel(KalmanGain) => numbers(math::kalman_gain(a, &array(b), &array(c))),
        Kernel(Innovation) => numbers(math::innovation(&array(a), &array(b))),
        Kernel(CorrectedState) => numbers(math::corrected_state(&array(a), &array(b), &array(c))),
        Kernel(JosephCovarianceUpdate) => numbers(math::joseph_covariance_update(
            a,
            &array(b),
            &array(c),
            &array(d),
        )),
        Kernel(CovarianceSymmetrization) => numbers(math::covariance_symmetrization(a)),
        Predicate(CandidateFinite) => ClosedEkfResult::Bool(math::candidate_finite(&array(a), b)),
        Predicate(CovariancePositiveDiagonal) => {
            ClosedEkfResult::Bool(math::covariance_positive_diagonal(a))
        }
        Predicate(CovarianceSymmetric) => ClosedEkfResult::Bool(math::covariance_symmetric(a)),
    })
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

use ChangeDetectionPolicy::{ExactScalar, KernelReported};
use EkfKernel::*;
use EkfPredicate::*;
use FrozenEkfOperation::{Kernel, Predicate};
use FrozenEkfValueShape::{Bool, F64, Matrix, Vector};

const V2: FrozenEkfValueShape = Vector(2);
const V3: FrozenEkfValueShape = Vector(3);
const V4: FrozenEkfValueShape = Vector(4);
const M2: FrozenEkfValueShape = Matrix {
    rows: 2,
    columns: 2,
};
const M3: FrozenEkfValueShape = Matrix {
    rows: 3,
    columns: 3,
};
const M2X3: FrozenEkfValueShape = Matrix {
    rows: 2,
    columns: 3,
};
const M3X2: FrozenEkfValueShape = Matrix {
    rows: 3,
    columns: 2,
};

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
