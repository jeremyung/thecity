use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Parcels and their street frontage. Flattened into the save, so its fields keep their top-level keys.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct LandState {
    pub(crate) parcels: Vec<Parcel>,
    #[serde(default)]
    pub(crate) parcel_frontages: Vec<ParcelFrontage>,
}
