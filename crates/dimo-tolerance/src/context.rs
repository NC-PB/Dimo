//! What the tolerance engine needs besides the callout: the project tolerance settings, the
//! tables and the units (FR-TOL-01, FR-TOL-07, FR-TOL-09, M2 decision 2).

use dimo_core::characteristic::Unit;
use dimo_core::derivation::TableRef;
use dimo_core::project::{TableClass, ToleranceSettings};

use crate::error::TableError;
use crate::format::{PartKind, TableKind};
use crate::load::{TableSet, load_custom_table};
use crate::table::Table;

/// Id of the fit table used when the set holds it (FR-TOL-04).
pub const DEFAULT_FIT_TABLE: &str = "iso-286";

/// Why a context could not be built. The project settings name something the tables do not
/// provide.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ContextError {
    /// The settings break one of their own rules ([`ToleranceSettings::validate`]).
    #[error("invalid tolerance settings: {0}")]
    Settings(&'static str),
    /// A table named in the settings is not in the set.
    #[error("table {0} is not available")]
    MissingTable(String),
    /// A table is available, but in another version than the settings name.
    #[error("table {id} is available in version {found}, the project needs version {expected}")]
    Version {
        /// Table id.
        id: String,
        /// Version named in the settings.
        expected: u32,
        /// Version in the set.
        found: u32,
    },
    /// A table is used for something its kind does not allow, e.g. a fit table as general
    /// tolerance or a shipped table as drawing rule.
    #[error("table {id} cannot be used as {usage}")]
    WrongKind {
        /// Table id.
        id: String,
        /// `general tolerance` or `drawing rule`.
        usage: &'static str,
    },
    /// No part of the table has the class column the settings name.
    #[error("table {id} has no class {class}")]
    UnknownClass {
        /// Table id.
        id: String,
        /// The class.
        class: String,
    },
    /// A stored custom table could not be loaded.
    #[error(transparent)]
    Table(#[from] TableError),
}

/// Settings, tables and units for [`crate::interpret`].
///
/// Build it once per project and sheet: [`ToleranceContext::new`] from settings and a table set
/// that already holds the project's custom tables, or [`ToleranceContext::for_project`] from
/// the shipped tables and the stored custom table files. Then set the sheet's unit with
/// [`ToleranceContext::with_drawing_unit`].
#[derive(Debug, Clone, PartialEq)]
pub struct ToleranceContext {
    settings: ToleranceSettings,
    tables: TableSet,
    fit_table: Option<String>,
    drawing_unit: Unit,
    output_unit: Option<Unit>,
}

impl ToleranceContext {
    /// A context for `settings` with the tables of `tables`. Checks that every table the
    /// settings name is in the set in the named version and kind, and that the classes exist.
    ///
    /// The fit table is `iso-286` if present, else the first fit table of the set. The drawing
    /// unit is mm and values stay in the unit they are written in, see
    /// [`ToleranceContext::with_drawing_unit`] and [`ToleranceContext::with_output_unit`].
    pub fn new(settings: &ToleranceSettings, tables: TableSet) -> Result<Self, ContextError> {
        settings.validate().map_err(ContextError::Settings)?;
        for entry in &settings.custom_tables {
            let table = find(&tables, &entry.table)?;
            if table.header().kind != TableKind::Custom {
                return Err(ContextError::WrongKind {
                    id: entry.table.id.clone(),
                    usage: "custom table",
                });
            }
        }
        if let Some(general) = &settings.general {
            check_class(
                &tables,
                general,
                &[TableKind::General, TableKind::Custom],
                "general tolerance",
            )?;
        }
        if let Some(rule) = &settings.drawing_rule {
            check_class(&tables, rule, &[TableKind::Custom], "drawing rule")?;
        }
        let fit_table = tables
            .get(DEFAULT_FIT_TABLE)
            .filter(|t| t.header().kind == TableKind::Fit)
            .or_else(|| {
                tables
                    .tables()
                    .iter()
                    .find(|t| t.header().kind == TableKind::Fit)
            })
            .map(|t| t.id().to_owned());
        Ok(Self {
            settings: settings.clone(),
            tables,
            fit_table,
            drawing_unit: Unit::Mm,
            output_unit: None,
        })
    }

    /// A context for a project: `base` (usually [`TableSet::shipped`]) plus the custom tables
    /// stored in the project. `stored` yields `(id, file text)` per stored table file; every
    /// table of `settings.custom_tables` must be among them, with id and version inside the
    /// file equal to the settings entry ([`load_custom_table`]).
    pub fn for_project<'a>(
        settings: &ToleranceSettings,
        mut base: TableSet,
        stored: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> Result<Self, ContextError> {
        let stored: Vec<(&str, &str)> = stored.into_iter().collect();
        for entry in &settings.custom_tables {
            let (_, text) = stored
                .iter()
                .find(|(id, _)| *id == entry.table.id)
                .ok_or_else(|| ContextError::MissingTable(entry.table.id.clone()))?;
            let origin = format!("project:tolerances/{}.toml", entry.table.id);
            base.add(load_custom_table(entry, &origin, text)?)?;
        }
        Self::new(settings, base)
    }

    /// The length unit of dimensions written without a unit: the unit of the sheet (D-20,
    /// never inferred from the sheet size). `Deg` is not a length unit and is ignored.
    #[must_use]
    pub fn with_drawing_unit(mut self, unit: Unit) -> Self {
        if unit.is_length() {
            self.drawing_unit = unit;
        }
        self
    }

    /// The length unit nominal and limits are stored in. `None` (the default) keeps the unit
    /// of the callout. Converted values are rounded as the settings say (FR-TOL-09).
    #[must_use]
    pub fn with_output_unit(mut self, unit: Option<Unit>) -> Self {
        self.output_unit = unit.filter(|u| u.is_length());
        self
    }

    /// The tolerance settings.
    pub fn settings(&self) -> &ToleranceSettings {
        &self.settings
    }

    /// The tables.
    pub fn tables(&self) -> &TableSet {
        &self.tables
    }

    /// The fit table, if the set holds one.
    pub fn fit_table(&self) -> Option<&Table> {
        self.fit_table.as_deref().and_then(|id| self.tables.get(id))
    }

    /// Length unit of dimensions written without a unit.
    pub fn drawing_unit(&self) -> Unit {
        self.drawing_unit
    }

    /// Length unit values are stored in; `None` keeps the unit of the callout.
    pub fn output_unit(&self) -> Option<Unit> {
        self.output_unit
    }

    /// The table of a table setting. Present for every setting: checked in
    /// [`ToleranceContext::new`].
    pub(crate) fn table(&self, class: &TableClass) -> Option<&Table> {
        self.tables.get(&class.table.id)
    }
}

fn find<'t>(tables: &'t TableSet, table: &TableRef) -> Result<&'t Table, ContextError> {
    let found = tables
        .get(&table.id)
        .ok_or_else(|| ContextError::MissingTable(table.id.clone()))?;
    if found.header().version != table.version {
        return Err(ContextError::Version {
            id: table.id.clone(),
            expected: table.version,
            found: found.header().version,
        });
    }
    Ok(found)
}

fn check_class(
    tables: &TableSet,
    setting: &TableClass,
    kinds: &[TableKind],
    usage: &'static str,
) -> Result<(), ContextError> {
    let table = find(tables, &setting.table)?;
    if !kinds.contains(&table.header().kind) {
        return Err(ContextError::WrongKind {
            id: setting.table.id.clone(),
            usage,
        });
    }
    let has_class = table
        .parts()
        .iter()
        .any(|p| p.data().kind == PartKind::Symmetric && p.column(&setting.class).is_some());
    if !has_class {
        return Err(ContextError::UnknownClass {
            id: setting.table.id.clone(),
            class: setting.class.clone(),
        });
    }
    Ok(())
}
