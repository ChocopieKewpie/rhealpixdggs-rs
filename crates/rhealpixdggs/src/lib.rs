//! Fast, dependency-free aperture-9 rHEALPix DGGS indexing.
//!
//! The core crate deliberately contains no Python or geometry-library types.
//! Language bindings can therefore share one implementation and one stable
//! integer/string cell-ID model.

mod bulk;
mod cell;
mod coverage;
mod dggs;
mod ellipsoid;
mod error;
mod projection;
mod topology;

pub use bulk::{
    BOUNDARY_PARALLEL_THRESHOLD, POINT_PARALLEL_THRESHOLD, REGION_PARALLEL_THRESHOLD,
    parallelism_available,
};
pub use cell::{
    CellId, CellShape, Direction, EllipsoidalDirection, Face, Region, compact_cells,
    uncompact_cells,
};
pub use dggs::RhealpixDggs;
pub use ellipsoid::{Ellipsoid, WGS84_A, WGS84_INVERSE_FLATTENING};
pub use error::{Error, Result};

/// Finest resolution supported by the stable 64-bit identifier encoding.
pub const MAX_RESOLUTION: u8 = 15;
