use std::fmt;

/// Errors returned by the rHEALPix core.
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// A cell identifier could not be parsed or violates the aperture-9 rules.
    InvalidCellId(String),
    /// A traversal index lies outside the finite cell hierarchy.
    InvalidCellIndex {
        /// Invalid zero-based index.
        index: u64,
        /// Traversal order used to interpret the index.
        order: &'static str,
    },
    /// The requested resolution is outside the supported range.
    InvalidResolution(u8),
    /// A requested parent resolution is finer than the cell.
    InvalidParentResolution {
        /// Requested ancestor resolution.
        requested: u8,
        /// Resolution of the input cell.
        cell: u8,
    },
    /// A requested descendant resolution is coarser than the cell.
    InvalidDescendantResolution {
        /// Requested descendant resolution.
        requested: u8,
        /// Resolution of the input cell.
        cell: u8,
    },
    /// Two topology inputs have different cell resolutions.
    ResolutionMismatch {
        /// Resolution of the origin cell.
        origin: u8,
        /// Resolution of the destination cell.
        destination: u8,
    },
    /// Geographic input was non-finite or outside its valid range.
    InvalidCoordinate(String),
    /// A line, polygon, or polygon hole is malformed.
    InvalidGeometry(String),
    /// A planar direction name was not one of left, right, down, or up.
    InvalidDirection(String),
    /// A geographic direction name is not valid for an ellipsoidal cell.
    InvalidEllipsoidalDirection(String),
    /// A densified boundary requested fewer than two points per edge.
    InvalidBoundaryPointCount(usize),
    /// Projected input does not lie inside the rHEALPix image.
    OutsideProjection,
    /// A bulk expansion would allocate an unreasonable number of cells.
    ExpansionTooLarge(u64),
    /// A grid traversal distance can exceed the allocation safety limit.
    GridDistanceTooLarge {
        /// Requested edge-neighbour distance.
        requested: u32,
        /// Largest accepted distance at this resolution.
        maximum: u32,
    },
    /// A boundary request would allocate an unreasonable number of points.
    BoundaryTooLarge(u64),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCellId(value) => write!(f, "invalid rHEALPix cell ID: {value}"),
            Self::InvalidCellIndex { index, order } => {
                write!(f, "invalid {order}-order cell index: {index}")
            }
            Self::InvalidResolution(value) => write!(
                f,
                "resolution {value} is invalid; expected 0..={}",
                crate::MAX_RESOLUTION
            ),
            Self::InvalidParentResolution { requested, cell } => write!(
                f,
                "parent resolution {requested} is finer than cell resolution {cell}"
            ),
            Self::InvalidDescendantResolution { requested, cell } => write!(
                f,
                "descendant resolution {requested} is coarser than cell resolution {cell}"
            ),
            Self::ResolutionMismatch {
                origin,
                destination,
            } => write!(
                f,
                "cells must have the same resolution; got {origin} and {destination}"
            ),
            Self::InvalidCoordinate(message) => write!(f, "invalid coordinate: {message}"),
            Self::InvalidGeometry(message) => write!(f, "invalid geometry: {message}"),
            Self::InvalidDirection(value) => write!(
                f,
                "invalid planar direction {value:?}; expected left, right, down, or up"
            ),
            Self::InvalidEllipsoidalDirection(value) => {
                write!(f, "invalid ellipsoidal neighbour direction {value:?}")
            }
            Self::InvalidBoundaryPointCount(value) => write!(
                f,
                "boundary points per edge must be at least 2; got {value}"
            ),
            Self::OutsideProjection => write!(f, "point lies outside the rHEALPix image"),
            Self::ExpansionTooLarge(count) => write!(
                f,
                "operation would produce {count} cells; split the request into smaller regions"
            ),
            Self::GridDistanceTooLarge { requested, maximum } => write!(
                f,
                "grid distance {requested} exceeds the maximum {maximum} at this resolution"
            ),
            Self::BoundaryTooLarge(count) => write!(
                f,
                "operation would produce {count} boundary points; reduce points_per_edge"
            ),
        }
    }
}

impl std::error::Error for Error {}

/// Convenient result alias for this crate.
pub type Result<T> = std::result::Result<T, Error>;
