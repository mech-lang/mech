use mech_core::CellSlotId;

use crate::efficacy::ekf::operation::{EkfConstants, EkfKernel};

pub(crate) const LOGICAL_SLOTS_PER_EKF: u32 = 17;
pub(crate) const NODES_PER_EKF: usize = 15;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SlotKind {
    Matrix2,
    Vector2,
    Vector3,
    Matrix3,
    Matrix2x3,
    Matrix3x2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SlotRole {
    Input,
    Stateful,
    Intermediate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SlotDecl {
    pub(crate) id: CellSlotId,
    pub(crate) kind: SlotKind,
    pub(crate) role: SlotRole,
    pub(crate) instance: u32,
}

#[derive(Clone, Debug)]
pub(crate) struct NodeDecl {
    pub(crate) op: EkfKernel,
    pub(crate) instance: u32,
    pub(crate) reads: Box<[CellSlotId]>,
    pub(crate) writes: Box<[CellSlotId]>,
}

#[derive(Clone, Debug)]
pub(crate) struct ProgramArtifact {
    pub(crate) instances: usize,
    pub(crate) slots: Box<[SlotDecl]>,
    pub(crate) nodes: Box<[NodeDecl]>,
    pub(crate) constants: EkfConstants,
}

pub(crate) mod slot {
    use mech_core::{CellSlotId, SlotIndex};

    use super::LOGICAL_SLOTS_PER_EKF;

    pub(crate) const STATE: u32 = 0;
    pub(crate) const COVARIANCE: u32 = 1;
    pub(crate) const INPUT: u32 = 2;
    pub(crate) const TRIG: u32 = 3;
    pub(crate) const MOTION_JACOBIAN: u32 = 4;
    pub(crate) const CONTROL_JACOBIAN: u32 = 5;
    pub(crate) const PREDICTED_STATE: u32 = 6;
    pub(crate) const PREDICTED_COVARIANCE: u32 = 7;
    pub(crate) const DELTA_RANGE: u32 = 8;
    pub(crate) const PREDICTED_MEASUREMENT: u32 = 9;
    pub(crate) const MEASUREMENT_JACOBIAN: u32 = 10;
    pub(crate) const INNOVATION_COVARIANCE: u32 = 11;
    pub(crate) const INVERSE_INNOVATION: u32 = 12;
    pub(crate) const GAIN: u32 = 13;
    pub(crate) const INNOVATION: u32 = 14;
    pub(crate) const CORRECTED_STATE: u32 = 15;
    pub(crate) const CORRECTED_COVARIANCE: u32 = 16;

    #[inline]
    pub(crate) const fn id(instance: u32, local: u32) -> CellSlotId {
        CellSlotId(instance * LOGICAL_SLOTS_PER_EKF + local)
    }

    #[inline]
    pub(crate) const fn index(instance: u32, local: u32) -> SlotIndex {
        SlotIndex(id(instance, local).0)
    }
}

fn node(instance: u32, op: EkfKernel, reads: &[u32], writes: &[u32]) -> NodeDecl {
    NodeDecl {
        op,
        instance,
        reads: reads
            .iter()
            .map(|local| slot::id(instance, *local))
            .collect(),
        writes: writes
            .iter()
            .map(|local| slot::id(instance, *local))
            .collect(),
    }
}

fn append_ekf_nodes(nodes: &mut Vec<NodeDecl>, instance: u32) {
    use EkfKernel::*;
    use slot::*;

    nodes.extend([
        node(instance, TrigonometricState, &[STATE], &[TRIG]),
        node(
            instance,
            MotionJacobian,
            &[STATE, INPUT, TRIG],
            &[MOTION_JACOBIAN],
        ),
        node(instance, ControlJacobian, &[TRIG], &[CONTROL_JACOBIAN]),
        node(
            instance,
            PredictedState,
            &[STATE, INPUT, TRIG],
            &[PREDICTED_STATE],
        ),
        node(
            instance,
            PredictedCovariance,
            &[COVARIANCE, MOTION_JACOBIAN, CONTROL_JACOBIAN],
            &[PREDICTED_COVARIANCE],
        ),
        node(
            instance,
            LandmarkDeltaAndRange,
            &[PREDICTED_STATE],
            &[DELTA_RANGE],
        ),
        node(
            instance,
            PredictedMeasurement,
            &[PREDICTED_STATE, DELTA_RANGE],
            &[PREDICTED_MEASUREMENT],
        ),
        node(
            instance,
            MeasurementJacobian,
            &[DELTA_RANGE],
            &[MEASUREMENT_JACOBIAN],
        ),
        node(
            instance,
            InnovationCovariance,
            &[PREDICTED_COVARIANCE, MEASUREMENT_JACOBIAN],
            &[INNOVATION_COVARIANCE],
        ),
        node(
            instance,
            Solve2x2,
            &[INNOVATION_COVARIANCE],
            &[INVERSE_INNOVATION],
        ),
        node(
            instance,
            KalmanGain,
            &[
                PREDICTED_COVARIANCE,
                MEASUREMENT_JACOBIAN,
                INVERSE_INNOVATION,
            ],
            &[GAIN],
        ),
        node(
            instance,
            Innovation,
            &[INPUT, PREDICTED_MEASUREMENT],
            &[INNOVATION],
        ),
        node(
            instance,
            CorrectedState,
            &[PREDICTED_STATE, GAIN, INNOVATION],
            &[CORRECTED_STATE],
        ),
        node(
            instance,
            JosephCovarianceUpdate,
            &[PREDICTED_COVARIANCE, MEASUREMENT_JACOBIAN, GAIN],
            &[CORRECTED_COVARIANCE],
        ),
        node(
            instance,
            CovarianceSymmetrization,
            &[CORRECTED_STATE, CORRECTED_COVARIANCE],
            &[STATE, COVARIANCE],
        ),
    ]);
}

impl ProgramArtifact {
    pub(crate) fn frozen_ekf_batch(instances: usize) -> Self {
        assert!(instances > 0, "resident EKF batch must not be empty");
        let slot_capacity = instances
            .checked_mul(LOGICAL_SLOTS_PER_EKF as usize)
            .expect("resident logical slot count");
        let node_capacity = instances
            .checked_mul(NODES_PER_EKF)
            .expect("resident node count");
        let mut slots = Vec::with_capacity(slot_capacity);
        let mut nodes = Vec::with_capacity(node_capacity);
        for instance in 0..instances {
            let instance = u32::try_from(instance).expect("resident instance count fits u32");
            use SlotKind::*;
            use SlotRole::*;
            let declarations = [
                (Vector3, Stateful),
                (Matrix3, Stateful),
                (Matrix2, Input),
                (Vector2, Intermediate),
                (Matrix3, Intermediate),
                (Matrix3x2, Intermediate),
                (Vector3, Intermediate),
                (Matrix3, Intermediate),
                (Vector3, Intermediate),
                (Vector2, Intermediate),
                (Matrix2x3, Intermediate),
                (Matrix2, Intermediate),
                (Matrix2, Intermediate),
                (Matrix3x2, Intermediate),
                (Vector2, Intermediate),
                (Vector3, Intermediate),
                (Matrix3, Intermediate),
            ];
            slots.extend(
                declarations
                    .into_iter()
                    .enumerate()
                    .map(|(local, (kind, role))| SlotDecl {
                        id: slot::id(instance, local as u32),
                        kind,
                        role,
                        instance,
                    }),
            );
            append_ekf_nodes(&mut nodes, instance);
        }
        debug_assert_eq!(slots.len(), slot_capacity);
        debug_assert_eq!(nodes.len(), node_capacity);
        Self {
            instances,
            slots: slots.into_boxed_slice(),
            nodes: nodes.into_boxed_slice(),
            constants: EkfConstants {
                dt: 0.05,
                landmark: [25.0, -10.0],
                process_covariance: [0.04, 0.0, 0.0, 0.0025],
                measurement_covariance: [0.25, 0.0, 0.0, 0.0009],
            },
        }
    }
}
