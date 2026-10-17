//! Pure planning and geometry: no engine state, no shared RNG. Everything here
//! is deterministic from its inputs and is identical on native and WASM.
pub mod blocks;
pub mod cross_section;
pub mod development;
pub mod frontage;
pub mod geometry;
pub mod kinds;
pub mod masterplan;
pub mod rail_alignment;
pub mod river_course;
pub mod road_curve;
