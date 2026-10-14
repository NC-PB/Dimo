//! Language of generated text: tolerance explanations, export headers and reports (D-32).
//!
//! One type for every place that writes words or numbers for a reader. Exports and reports pick
//! it per export, independent of the UI language (D-32); explanations follow the UI language.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Language of generated text. Serialized as the two letter code (`en`, `de`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum Language {
    /// English: decimal point.
    #[default]
    En,
    /// German: decimal comma.
    De,
}

impl Language {
    /// Two letter code (`en`, `de`), the same as the serialized form.
    pub fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::De => "de",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_is_the_serialized_form() {
        for language in [Language::En, Language::De] {
            assert_eq!(
                serde_json::to_value(language).unwrap(),
                serde_json::Value::from(language.code())
            );
            let back: Language =
                serde_json::from_value(serde_json::Value::from(language.code())).unwrap();
            assert_eq!(back, language);
        }
        assert_eq!(Language::default(), Language::En);
    }
}
