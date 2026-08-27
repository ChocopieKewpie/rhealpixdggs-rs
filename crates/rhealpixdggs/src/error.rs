use std::fmt;

/// Errors returned by the rHEALPix core.
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// A cell identifier could not be parsed or violates the aperture-9 rules.
    InvalidCellId(String),
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
    /// Geographic input was non-finite or outside its valid range.
    InvalidCoordinate(String),
    /// Projected input does not lie inside the rHEALPix image.
    OutsideProjection,
    /// A bulk expansion would allocate an unreasonable number of cells.
    ExpansionTooLarge(u64),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCellId(value) => write!(f, "invalid rHEALPix cell ID: {value}"),
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
            Self::InvalidCoordinate(message) => write!(f, "invalid coordinate: {message}"),
            Self::OutsideProjection => write!(f, "point lies outside the rHEALPix image"),
            Self::ExpansionTooLarge(count) => write!(
                f,
                "operation would produce {count} cells; split the request into smaller regions"
            ),
        }
    }
}

impl std::error::Error for Error {}

/// Convenient result alias for this crate.
pub type Result<T> = std::result::Result<T, Error>;
