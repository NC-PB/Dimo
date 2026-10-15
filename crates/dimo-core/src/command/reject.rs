//! Machine readable reasons of refused commands, for the frontend to translate (MF F3).

use serde::{Deserialize, Serialize};

use super::CommandError;
use crate::characteristic::FieldError;
use crate::measurement::SerialError;

/// Why a document command was refused: the variants of [`CommandError`] the user can run into,
/// without their details. [`CommandError::reject_reason`] is the one mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum RejectReason {
    /// A characteristic of the command does not exist.
    UnknownCharacteristic,
    /// A balloon of the command does not exist.
    UnknownBalloon,
    /// A sheet of the command does not exist.
    UnknownSheet,
    /// The command would change locked numbers (D-23).
    NumberingLocked,
    /// A position or region is not valid.
    InvalidGeometry,
    /// A balloon style size is not valid.
    InvalidStyle,
    /// A sheet unit, scale, zone grid or view is not valid.
    InvalidSheetSetting,
    /// Project settings break their rules, e.g. tolerance or numbering settings.
    InvalidProjectSetting,
    /// Characteristics cannot be moved before one of themselves.
    InvalidMoveTarget,
    /// Quantity 0.
    ZeroQuantity,
    /// A sample of the command does not exist.
    UnknownSample,
    /// A drawing revision of the command does not exist.
    UnknownRevision,
    /// The drawing revision is a PDF, so it has no image edit settings (MF F1).
    NotAnImage,
    /// Image edit settings are out of range (MF F1).
    InvalidImageEdit,
    /// A sample serial is empty (M3 decision 6).
    EmptySerial,
    /// Another sample has this serial (M3 decision 6).
    DuplicateSerial,
    /// A project holds at most 500 samples (M3 decision 6).
    TooManySamples,
    /// Samples are added 1 to 100 at once (M3 decision 6).
    InvalidSampleCount,
    /// A date is neither empty nor `YYYY-MM-DD`.
    InvalidDate,
    /// A typed result does not fit the result type of its characteristic (M3 decisions 7, 8).
    InvalidResult,
    /// The result type cannot change while results of the other type exist (M3 decision 4).
    ResultTypeConflict,
    /// A custom tolerance table of the settings is not stored in the project (M2 decision 4).
    MissingTable,
    /// Undo without history.
    NothingToUndo,
    /// Redo without history.
    NothingToRedo,
    /// Internal inconsistency; a bug.
    Internal,
}

impl CommandError {
    /// The machine readable reason of this refusal (MF F3).
    pub fn reject_reason(&self) -> RejectReason {
        use RejectReason as R;
        match self {
            Self::UnknownCharacteristic(_) => R::UnknownCharacteristic,
            Self::UnknownBalloon(_) => R::UnknownBalloon,
            Self::UnknownSheet(_) => R::UnknownSheet,
            Self::NumberingLocked => R::NumberingLocked,
            Self::InvalidGeometry(_) => R::InvalidGeometry,
            Self::InvalidStyle => R::InvalidStyle,
            Self::InvalidSheetSetting(_) => R::InvalidSheetSetting,
            Self::InvalidProjectSetting(_) => R::InvalidProjectSetting,
            Self::InvalidMoveTarget => R::InvalidMoveTarget,
            Self::Field(FieldError::ZeroQuantity) => R::ZeroQuantity,
            Self::UnknownSample(_) => R::UnknownSample,
            Self::UnknownRevision(_) => R::UnknownRevision,
            Self::NotAnImage(_) => R::NotAnImage,
            Self::InvalidImageEdit(_) => R::InvalidImageEdit,
            Self::Serial(SerialError::Empty) => R::EmptySerial,
            Self::Serial(SerialError::Duplicate(_)) => R::DuplicateSerial,
            Self::TooManySamples => R::TooManySamples,
            Self::InvalidSampleCount => R::InvalidSampleCount,
            Self::InvalidDate(_) => R::InvalidDate,
            Self::InvalidResult { .. } => R::InvalidResult,
            Self::ResultTypeConflict(_) => R::ResultTypeConflict,
            Self::MissingTable(_) => R::MissingTable,
            Self::NothingToUndo => R::NothingToUndo,
            Self::NothingToRedo => R::NothingToRedo,
            Self::Change(_) => R::Internal,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::RevisionId;

    #[test]
    fn refusals_map_to_their_reason() {
        assert_eq!(
            CommandError::NumberingLocked.reject_reason(),
            RejectReason::NumberingLocked
        );
        assert_eq!(
            CommandError::NotAnImage(RevisionId::from_uuid(uuid::Uuid::nil())).reject_reason(),
            RejectReason::NotAnImage
        );
        assert_eq!(
            CommandError::MissingTable("shop".to_owned()).reject_reason(),
            RejectReason::MissingTable
        );
        assert_eq!(
            CommandError::Field(FieldError::ZeroQuantity).reject_reason(),
            RejectReason::ZeroQuantity
        );
        assert_eq!(
            CommandError::Serial(SerialError::Duplicate("7".to_owned())).reject_reason(),
            RejectReason::DuplicateSerial
        );
        assert_eq!(
            serde_json::to_value(RejectReason::NumberingLocked).unwrap(),
            "numbering_locked"
        );
    }
}
