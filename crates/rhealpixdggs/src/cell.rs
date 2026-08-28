use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;

use crate::MAX_RESOLUTION;
use crate::error::{Error, Result};

const APERTURE: u64 = 9;
const MAX_EXPANSION: u64 = 10_000_000;

/// One of the six resolution-zero rHEALPix faces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Face {
    /// North polar face.
    N = 0,
    /// First equatorial face.
    O = 1,
    /// Second equatorial face.
    P = 2,
    /// Third equatorial face.
    Q = 3,
    /// Fourth equatorial face.
    R = 4,
    /// South polar face.
    S = 5,
}

impl Face {
    /// Construct a face from its zero-based number.
    pub const fn from_number(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::N),
            1 => Some(Self::O),
            2 => Some(Self::P),
            3 => Some(Self::Q),
            4 => Some(Self::R),
            5 => Some(Self::S),
            _ => None,
        }
    }

    /// Return the zero-based face number.
    pub const fn number(self) -> u8 {
        self as u8
    }

    /// Return the canonical face letter.
    pub const fn letter(self) -> char {
        match self {
            Self::N => 'N',
            Self::O => 'O',
            Self::P => 'P',
            Self::Q => 'Q',
            Self::R => 'R',
            Self::S => 'S',
        }
    }
}

impl TryFrom<char> for Face {
    type Error = Error;

    fn try_from(value: char) -> Result<Self> {
        match value {
            'N' => Ok(Self::N),
            'O' => Ok(Self::O),
            'P' => Ok(Self::P),
            'Q' => Ok(Self::Q),
            'R' => Ok(Self::R),
            'S' => Ok(Self::S),
            _ => Err(Error::InvalidCellId(value.to_string())),
        }
    }
}

/// A compact aperture-9 rHEALPix spatially unique identifier.
///
/// The string representation is compatible with `rhealpixdggs-py`, for
/// example `Q381`. Resolution-zero cells contain only their face letter.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CellId {
    face: Face,
    digits: Vec<u8>,
}

impl CellId {
    /// Build a validated cell identifier from a face and row-major child digits.
    pub fn new(face: Face, digits: Vec<u8>) -> Result<Self> {
        let resolution =
            u8::try_from(digits.len()).map_err(|_| Error::InvalidResolution(u8::MAX))?;
        validate_resolution(resolution)?;
        if digits.iter().any(|digit| *digit >= APERTURE as u8) {
            return Err(Error::InvalidCellId(format!(
                "{}{}",
                face.letter(),
                digits.iter().map(u8::to_string).collect::<String>()
            )));
        }
        Ok(Self { face, digits })
    }

    /// Return the resolution-zero face.
    pub const fn face(&self) -> Face {
        self.face
    }

    /// Return the child digits.
    pub fn digits(&self) -> &[u8] {
        &self.digits
    }

    /// Return this cell's resolution.
    pub fn resolution(&self) -> u8 {
        self.digits.len() as u8
    }

    /// Return this cell's direct parent, or `None` for a resolution-zero cell.
    pub fn parent(&self) -> Option<Self> {
        let mut digits = self.digits.clone();
        digits.pop()?;
        Some(Self {
            face: self.face,
            digits,
        })
    }

    /// Return the ancestor at `resolution`.
    pub fn parent_at(&self, resolution: u8) -> Result<Self> {
        validate_resolution(resolution)?;
        if resolution > self.resolution() {
            return Err(Error::InvalidParentResolution {
                requested: resolution,
                cell: self.resolution(),
            });
        }
        Ok(Self {
            face: self.face,
            digits: self.digits[..usize::from(resolution)].to_vec(),
        })
    }

    /// Return one direct child numbered `0..=8` in row-major order.
    pub fn child(&self, digit: u8) -> Result<Self> {
        if digit >= APERTURE as u8 {
            return Err(Error::InvalidCellId(format!("child digit {digit}")));
        }
        let next = self.resolution().saturating_add(1);
        validate_resolution(next)?;
        let mut digits = self.digits.clone();
        digits.push(digit);
        Ok(Self {
            face: self.face,
            digits,
        })
    }

    /// Return the nine direct children in row-major order.
    pub fn children(&self) -> Result<Vec<Self>> {
        (0..APERTURE as u8).map(|digit| self.child(digit)).collect()
    }

    /// Return whether `self` contains `other`, including equality.
    pub fn contains(&self, other: &Self) -> bool {
        self.face == other.face && other.digits.starts_with(&self.digits)
    }

    /// Return the number of descendants at `resolution`.
    pub fn descendant_count(&self, resolution: u8) -> Result<u64> {
        validate_resolution(resolution)?;
        if resolution < self.resolution() {
            return Err(Error::InvalidDescendantResolution {
                requested: resolution,
                cell: self.resolution(),
            });
        }
        Ok(APERTURE.pow(u32::from(resolution - self.resolution())))
    }

    /// Return all descendants at `resolution` in identifier order.
    pub fn descendants(&self, resolution: u8) -> Result<Vec<Self>> {
        let count = self.descendant_count(resolution)?;
        if count > MAX_EXPANSION {
            return Err(Error::ExpansionTooLarge(count));
        }
        let mut current = vec![self.clone()];
        for _ in self.resolution()..resolution {
            let mut next = Vec::with_capacity(current.len() * APERTURE as usize);
            for cell in current {
                next.extend(cell.children()?);
            }
            current = next;
        }
        Ok(current)
    }

    /// Encode the cell as a stable resolution-major integer.
    ///
    /// This encoding is intended as a language-neutral interchange form. It
    /// covers all WGS84_003 cells through resolution 15 in fewer than 51 bits.
    pub fn to_u64(&self) -> u64 {
        let resolution = self.resolution();
        let cells_per_face = APERTURE.pow(u32::from(resolution));
        let mut within_face = 0_u64;
        for digit in &self.digits {
            within_face = within_face * APERTURE + u64::from(*digit);
        }
        resolution_offset(resolution) + u64::from(self.face.number()) * cells_per_face + within_face
    }

    /// Decode a stable resolution-major integer.
    pub fn from_u64(value: u64) -> Result<Self> {
        let mut resolution = None;
        for candidate in 0..=MAX_RESOLUTION {
            if value < resolution_offset(candidate.saturating_add(1)) {
                resolution = Some(candidate);
                break;
            }
        }
        let resolution = resolution.ok_or_else(|| Error::InvalidCellId(value.to_string()))?;
        let cells_per_face = APERTURE.pow(u32::from(resolution));
        let within_resolution = value - resolution_offset(resolution);
        let face_number = within_resolution / cells_per_face;
        let face = Face::from_number(face_number as u8)
            .ok_or_else(|| Error::InvalidCellId(value.to_string()))?;
        let mut remainder = within_resolution % cells_per_face;
        let mut digits = vec![0; usize::from(resolution)];
        for digit in digits.iter_mut().rev() {
            *digit = (remainder % APERTURE) as u8;
            remainder /= APERTURE;
        }
        Self::new(face, digits)
    }
}

impl fmt::Display for CellId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.face.letter())?;
        for digit in &self.digits {
            write!(f, "{digit}")?;
        }
        Ok(())
    }
}

impl FromStr for CellId {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        let mut chars = value.chars();
        let face = chars
            .next()
            .ok_or_else(|| Error::InvalidCellId(value.to_owned()))
            .and_then(Face::try_from)?;
        let digits = chars
            .map(|character| {
                character
                    .to_digit(10)
                    .filter(|digit| *digit < APERTURE as u32)
                    .map(|digit| digit as u8)
                    .ok_or_else(|| Error::InvalidCellId(value.to_owned()))
            })
            .collect::<Result<Vec<_>>>()?;
        Self::new(face, digits)
    }
}

/// Recursively replace complete groups of nine siblings with their parent.
pub fn compact_cells<I>(cells: I) -> Vec<CellId>
where
    I: IntoIterator<Item = CellId>,
{
    let mut cells: BTreeSet<CellId> = cells.into_iter().collect();

    // An explicit ancestor makes any included descendants redundant.
    let snapshot: Vec<_> = cells.iter().cloned().collect();
    for cell in snapshot {
        let mut ancestor = cell.parent();
        while let Some(parent) = ancestor {
            if cells.contains(&parent) {
                cells.remove(&cell);
                break;
            }
            ancestor = parent.parent();
        }
    }

    loop {
        let mut groups: BTreeMap<CellId, u8> = BTreeMap::new();
        for cell in &cells {
            if let Some(parent) = cell.parent() {
                *groups.entry(parent).or_default() += 1;
            }
        }
        let complete: Vec<_> = groups
            .into_iter()
            .filter_map(|(parent, count)| (count == APERTURE as u8).then_some(parent))
            .collect();
        if complete.is_empty() {
            break;
        }
        for parent in complete {
            for child in parent.children().expect("parent below max resolution") {
                cells.remove(&child);
            }
            cells.insert(parent);
        }
    }
    cells.into_iter().collect()
}

/// Expand cells to a common resolution, removing duplicate output cells.
pub fn uncompact_cells<I>(cells: I, resolution: u8) -> Result<Vec<CellId>>
where
    I: IntoIterator<Item = CellId>,
{
    validate_resolution(resolution)?;
    let cells: BTreeSet<CellId> = cells.into_iter().collect();
    let mut total = 0_u64;
    for cell in &cells {
        total = total
            .checked_add(cell.descendant_count(resolution)?)
            .ok_or(Error::ExpansionTooLarge(u64::MAX))?;
    }
    if total > MAX_EXPANSION {
        return Err(Error::ExpansionTooLarge(total));
    }
    let mut result = BTreeSet::new();
    for cell in cells {
        result.extend(cell.descendants(resolution)?);
    }
    Ok(result.into_iter().collect())
}

pub(crate) fn validate_resolution(resolution: u8) -> Result<()> {
    if resolution > MAX_RESOLUTION {
        return Err(Error::InvalidResolution(resolution));
    }
    Ok(())
}

const fn resolution_offset(resolution: u8) -> u64 {
    if resolution == 0 {
        0
    } else {
        6 * (APERTURE.pow(resolution as u32) - 1) / (APERTURE - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_formats_upstream_ids() {
        let cell: CellId = "N038".parse().unwrap();
        assert_eq!(cell.face(), Face::N);
        assert_eq!(cell.digits(), &[0, 3, 8]);
        assert_eq!(cell.to_string(), "N038");
        assert!("A0".parse::<CellId>().is_err());
        assert!("N9".parse::<CellId>().is_err());
    }

    #[test]
    fn integer_encoding_round_trips_every_test_cell() {
        for face in 0..6 {
            for digits in [vec![], vec![0], vec![8], vec![1, 4, 7], vec![8; 15]] {
                let cell = CellId::new(Face::from_number(face).unwrap(), digits).unwrap();
                assert_eq!(CellId::from_u64(cell.to_u64()).unwrap(), cell);
            }
        }
        assert!(CellId::from_u64(resolution_offset(MAX_RESOLUTION + 1)).is_err());
    }

    #[test]
    fn hierarchy_is_prefix_based() {
        let parent: CellId = "Q38".parse().unwrap();
        let child: CellId = "Q381".parse().unwrap();
        assert!(parent.contains(&child));
        assert_eq!(child.parent().unwrap(), parent);
        assert_eq!(child.parent_at(0).unwrap().to_string(), "Q");
        assert_eq!(parent.children().unwrap().len(), 9);
    }

    #[test]
    fn compaction_is_recursive() {
        let root: CellId = "P".parse().unwrap();
        let grandchildren = root.descendants(2).unwrap();
        assert_eq!(compact_cells(grandchildren), vec![root]);
    }

    #[test]
    fn uncompaction_round_trips() {
        let root: CellId = "P".parse().unwrap();
        let expanded = uncompact_cells([root.clone()], 2).unwrap();
        assert_eq!(expanded.len(), 81);
        assert_eq!(compact_cells(expanded), vec![root]);
    }
}
