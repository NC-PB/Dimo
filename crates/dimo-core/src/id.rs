//! Stable IDs of the domain model (FR-BAL-09).
//!
//! Every entity has a UUID that never changes, independent of its display number. Each kind of
//! entity has its own newtype, so a balloon ID can never be passed where a characteristic ID is
//! expected. IDs are never generated inside `dimo-core`: they come from an
//! [`Environment`](crate::env::Environment), which keeps commands deterministic (rule 11).

use std::fmt;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! id_type {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        ///
        /// Serialized as a hyphenated lowercase UUID string.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[cfg_attr(feature = "specta", derive(specta::Type))]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            /// Wraps a UUID from an ID provider or a stored project.
            pub const fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            /// The wrapped UUID, for storage layers that need the raw value.
            pub const fn as_uuid(&self) -> &Uuid {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0.hyphenated(), f)
            }
        }

        impl JsonSchema for $name {
            fn schema_name() -> std::borrow::Cow<'static, str> {
                stringify!($name).into()
            }

            fn json_schema(_: &mut SchemaGenerator) -> Schema {
                json_schema!({ "type": "string", "format": "uuid" })
            }
        }
    };
}

id_type!(
    /// ID of a [`Characteristic`](crate::characteristic::Characteristic).
    CharId
);
id_type!(
    /// ID of a [`Balloon`](crate::balloon::Balloon).
    BalloonId
);
id_type!(
    /// ID of a [`Sheet`](crate::sheet::Sheet). Unique across all drawing revisions.
    SheetId
);
id_type!(
    /// ID of a [`DrawingRevision`](crate::sheet::DrawingRevision).
    RevisionId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_serialize_as_plain_uuid_strings() {
        let id = CharId::from_uuid(Uuid::from_u128(0x1234));
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"00000000-0000-0000-0000-000000001234\"");
        assert_eq!(serde_json::from_str::<CharId>(&json).unwrap(), id);
        assert_eq!(id.to_string(), "00000000-0000-0000-0000-000000001234");
    }
}
