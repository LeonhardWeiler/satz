use crate::color::{Color, Swatch};
use crate::text::TextStyle;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Chosen mode per collection; collections not listed use their first mode.
pub type Modes = BTreeMap<String, String>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mode {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub modes: Vec<Mode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Value {
    Color(Color),
    Number(f64),
}

/// `values` holds one value per mode of its collection, all of one kind.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Variable {
    pub id: String,
    pub collection: String,
    pub name: String,
    pub values: BTreeMap<String, Value>,
}

/// What colours, bound numbers and styled text refer to.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Palette {
    pub swatches: Vec<Swatch>,
    pub collections: Vec<Collection>,
    pub variables: Vec<Variable>,
    pub text_styles: Vec<TextStyle>,
}

/// A palette seen from a node with its modes.
#[derive(Clone, Copy)]
pub struct Scope<'a> {
    pub palette: &'a Palette,
    pub modes: &'a Modes,
}

impl Scope<'_> {
    pub fn value(&self, variable: &str) -> Option<&Value> {
        let v = self.palette.variables.iter().find(|v| v.id == variable)?;
        let c = self
            .palette
            .collections
            .iter()
            .find(|c| c.id == v.collection)?;
        let chosen = self.modes.get(&c.id);
        let mode = c
            .modes
            .iter()
            .find(|m| Some(&m.id) == chosen)
            .or(c.modes.first())?;
        v.values.get(&mode.id)
    }

    pub fn swatches(&self) -> &[Swatch] {
        &self.palette.swatches
    }
}

impl Value {
    pub fn same_kind(&self, other: &Value) -> bool {
        matches!(
            (self, other),
            (Value::Color(_), Value::Color(_)) | (Value::Number(_), Value::Number(_))
        )
    }
}
