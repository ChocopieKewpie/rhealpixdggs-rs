use std::cmp::Ordering;
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

/// Broad geographic region occupied by a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Region {
    /// The north-polar rHEALPix face.
    NorthPolar,
    /// One of the four equatorial faces.
    Equatorial,
    /// The south-polar rHEALPix face.
    SouthPolar,
}

impl Region {
    /// Return the name used by `rhealpixdggs-py`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NorthPolar => "north_polar",
            Self::Equatorial => "equatorial",
            Self::SouthPolar => "south_polar",
        }
    }
}

/// Shape of a cell after inverse projection onto the ellipsoid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CellShape {
    /// An equatorial quadrilateral.
    Quad,
    /// A polar cap containing a pole.
    Cap,
    /// A polar triangular dart.
    Dart,
    /// A polar skew quadrilateral.
    SkewQuad,
}

impl CellShape {
    /// Return the name used by `rhealpixdggs-py`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Quad => "quad",
            Self::Cap => "cap",
            Self::Dart => "dart",
            Self::SkewQuad => "skew_quad",
        }
    }
}

/// Cardinal direction on the rHEALPix unfolded plane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Direction {
    /// Decreasing planar x.
    Left,
    /// Increasing planar x.
    Right,
    /// Decreasing planar y.
    Down,
    /// Increasing planar y.
    Up,
}

impl Direction {
    /// All planar directions in upstream dictionary order.
    pub const ALL: [Self; 4] = [Self::Left, Self::Right, Self::Down, Self::Up];

    /// Return the upstream-compatible name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Down => "down",
            Self::Up => "up",
        }
    }
}

impl FromStr for Direction {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "left" => Ok(Self::Left),
            "right" => Ok(Self::Right),
            "down" => Ok(Self::Down),
            "up" => Ok(Self::Up),
            _ => Err(Error::InvalidDirection(value.to_owned())),
        }
    }
}

/// Geographic edge-neighbour direction on the ellipsoid.
///
/// The valid names depend on cell shape. Cap cells use indexed poleward
/// directions, while darts use diagonal names on their equatorward side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EllipsoidalDirection {
    /// Geographic north.
    North,
    /// Geographic south.
    South,
    /// Geographic east.
    East,
    /// Geographic west.
    West,
    /// Geographic northeast, used by southern darts.
    NorthEast,
    /// Geographic northwest, used by southern darts.
    NorthWest,
    /// Geographic southeast, used by northern darts.
    SouthEast,
    /// Geographic southwest, used by northern darts.
    SouthWest,
    /// Indexed northward neighbour of a southern cap.
    NorthIndexed(u8),
    /// Indexed southward neighbour of a northern cap.
    SouthIndexed(u8),
}

impl fmt::Display for EllipsoidalDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::North => f.write_str("north"),
            Self::South => f.write_str("south"),
            Self::East => f.write_str("east"),
            Self::West => f.write_str("west"),
            Self::NorthEast => f.write_str("north_east"),
            Self::NorthWest => f.write_str("north_west"),
            Self::SouthEast => f.write_str("south_east"),
            Self::SouthWest => f.write_str("south_west"),
            Self::NorthIndexed(index) => write!(f, "north_{index}"),
            Self::SouthIndexed(index) => write!(f, "south_{index}"),
        }
    }
}

impl FromStr for EllipsoidalDirection {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "north" => Ok(Self::North),
            "south" => Ok(Self::South),
            "east" => Ok(Self::East),
            "west" => Ok(Self::West),
            "north_east" => Ok(Self::NorthEast),
            "north_west" => Ok(Self::NorthWest),
            "south_east" => Ok(Self::SouthEast),
            "south_west" => Ok(Self::SouthWest),
            "north_0" => Ok(Self::NorthIndexed(0)),
            "north_1" => Ok(Self::NorthIndexed(1)),
            "north_2" => Ok(Self::NorthIndexed(2)),
            "north_3" => Ok(Self::NorthIndexed(3)),
            "south_0" => Ok(Self::SouthIndexed(0)),
            "south_1" => Ok(Self::SouthIndexed(1)),
            "south_2" => Ok(Self::SouthIndexed(2)),
            "south_3" => Ok(Self::SouthIndexed(3)),
            _ => Err(Error::InvalidEllipsoidalDirection(value.to_owned())),
        }
    }
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

    /// Return the upstream geographic region classification.
    pub const fn region(&self) -> Region {
        match self.face {
            Face::N => Region::NorthPolar,
            Face::S => Region::SouthPolar,
            Face::O | Face::P | Face::Q | Face::R => Region::Equatorial,
        }
    }

    /// Return the upstream ellipsoidal shape classification.
    pub fn shape(&self) -> CellShape {
        if self.region() == Region::Equatorial {
            return CellShape::Quad;
        }
        if self.digits.iter().all(|digit| *digit == 4) {
            return CellShape::Cap;
        }
        if self.digits.iter().all(|digit| matches!(digit, 0 | 4 | 8))
            || self.digits.iter().all(|digit| matches!(digit, 2 | 4 | 6))
        {
            return CellShape::Dart;
        }
        CellShape::SkewQuad
    }

    /// Rotate every child digit anticlockwise by quarter turns.
    ///
    /// The resolution-zero face remains fixed. This is the transformation
    /// used when planar neighbours cross a folded polar-face boundary.
    pub fn rotated(&self, quarter_turns: u8) -> Self {
        let turns = quarter_turns % 4;
        if turns == 0 {
            return self.clone();
        }
        let digits = self
            .digits
            .iter()
            .map(|digit| {
                let mut row = digit / 3;
                let mut column = digit % 3;
                for _ in 0..turns {
                    (row, column) = (column, 2 - row);
                }
                row * 3 + column
            })
            .collect();
        Self {
            face: self.face,
            digits,
        }
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

    /// Return the next cell at the same resolution in identifier order.
    pub fn successor(&self) -> Option<Self> {
        let mut digits = self.digits.clone();
        if let Some(index) = digits.iter().rposition(|digit| *digit < APERTURE as u8 - 1) {
            digits[index] += 1;
            digits[index + 1..].fill(0);
            return Some(Self {
                face: self.face,
                digits,
            });
        }

        Face::from_number(self.face.number() + 1).map(|face| Self {
            face,
            digits: vec![0; self.digits.len()],
        })
    }

    /// Return the greatest cell at `resolution` that precedes this cell.
    ///
    /// This follows upstream post-order traversal semantics. If the target is
    /// coarser, this cell is first truncated; if it is finer, its final
    /// descendant is returned.
    pub fn predecessor_at(&self, resolution: u8) -> Result<Option<Self>> {
        validate_resolution(resolution)?;
        match resolution.cmp(&self.resolution()) {
            Ordering::Less => Ok(self.parent_at(resolution)?.predecessor()),
            Ordering::Equal => Ok(self.predecessor()),
            Ordering::Greater => {
                let mut digits = self.digits.clone();
                digits.resize(usize::from(resolution), APERTURE as u8 - 1);
                Ok(Some(Self {
                    face: self.face,
                    digits,
                }))
            }
        }
    }

    /// Return the previous cell at the same resolution in identifier order.
    pub fn predecessor(&self) -> Option<Self> {
        let mut digits = self.digits.clone();
        if let Some(index) = digits.iter().rposition(|digit| *digit > 0) {
            digits[index] -= 1;
            digits[index + 1..].fill(APERTURE as u8 - 1);
            return Some(Self {
                face: self.face,
                digits,
            });
        }

        self.face
            .number()
            .checked_sub(1)
            .and_then(Face::from_number)
            .map(|face| Self {
                face,
                digits: vec![APERTURE as u8 - 1; self.digits.len()],
            })
    }

    /// Return the least cell at `resolution` that follows this cell.
    ///
    /// This follows upstream post-order traversal semantics. If the target is
    /// coarser, this cell is first truncated; if it is finer, the first cell
    /// in the next same-resolution subtree is returned. The terminal southern
    /// cell returns `None` at every requested resolution.
    pub fn successor_at(&self, resolution: u8) -> Result<Option<Self>> {
        validate_resolution(resolution)?;
        match resolution.cmp(&self.resolution()) {
            Ordering::Less => Ok(self.parent_at(resolution)?.successor()),
            Ordering::Equal => Ok(self.successor()),
            Ordering::Greater => Ok(self.successor().map(|successor| {
                let mut digits = successor.digits;
                digits.resize(usize::from(resolution), 0);
                Self {
                    face: successor.face,
                    digits,
                }
            })),
        }
    }

    /// Return the zero-based level-order index used by the stable integer ID.
    pub fn level_order_index(&self) -> u64 {
        self.to_u64()
    }

    /// Construct a cell from its zero-based level-order index.
    pub fn from_level_order_index(index: u64) -> Result<Self> {
        Self::from_u64(index).map_err(|_| Error::InvalidCellIndex {
            index,
            order: "level",
        })
    }

    /// Return the zero-based post-order index in the complete hierarchy.
    ///
    /// Descendants precede their parent, child subtrees are ordered `0..=8`,
    /// and the hierarchy is finite at [`crate::MAX_RESOLUTION`].
    pub fn post_order_index(&self) -> u64 {
        let mut result = u64::from(self.face.number()) * post_order_subtree_size(0);
        for (position, digit) in self.digits.iter().enumerate() {
            result += u64::from(*digit) * post_order_subtree_size(position as u8 + 1);
        }
        result + post_order_subtree_size(self.resolution()) - 1
    }

    /// Construct a cell from its zero-based post-order index.
    pub fn from_post_order_index(index: u64) -> Result<Self> {
        let root_size = post_order_subtree_size(0);
        let total = 6 * root_size;
        if index >= total {
            return Err(Error::InvalidCellIndex {
                index,
                order: "post",
            });
        }

        let face = Face::from_number((index / root_size) as u8)
            .expect("a validated post-order index has one of six faces");
        let mut remainder = index % root_size;
        let mut digits = Vec::new();
        for resolution in 0..=MAX_RESOLUTION {
            let subtree_size = post_order_subtree_size(resolution);
            if remainder == subtree_size - 1 {
                return Ok(Self { face, digits });
            }
            let child_size = post_order_subtree_size(resolution + 1);
            digits.push((remainder / child_size) as u8);
            remainder %= child_size;
        }
        unreachable!("the finest-level subtree contains exactly one cell")
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

impl Ord for CellId {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.face.cmp(&other.face) {
            Ordering::Equal => {
                for (left, right) in self.digits.iter().zip(&other.digits) {
                    match left.cmp(right) {
                        Ordering::Equal => {}
                        ordering => return ordering,
                    }
                }
                // Post-order traversal visits descendants before their parent.
                other.digits.len().cmp(&self.digits.len())
            }
            ordering => ordering,
        }
    }
}

impl PartialOrd for CellId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
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

const fn post_order_subtree_size(resolution: u8) -> u64 {
    (APERTURE.pow((MAX_RESOLUTION - resolution + 1) as u32) - 1) / (APERTURE - 1)
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
    fn level_and_post_order_indices_match_the_finite_hierarchy() {
        let cases = [
            ("N", 0, 231_627_523_606_479),
            ("N0", 6, 25_736_391_511_830),
            ("N2", 8, 77_209_174_535_492),
            ("N82", 134, 214_469_929_265_257),
            ("Q381", 3_049, 795_604_004_266_974),
            ("S", 5, 1_389_765_141_638_879),
            (
                "S888888888888888",
                1_389_765_141_638_879,
                1_389_765_141_638_864,
            ),
        ];
        for (identifier, level, post) in cases {
            let cell: CellId = identifier.parse().unwrap();
            assert_eq!(cell.level_order_index(), level, "level {identifier}");
            assert_eq!(cell.post_order_index(), post, "post {identifier}");
            assert_eq!(
                CellId::from_level_order_index(level).unwrap(),
                cell,
                "level round trip {identifier}"
            );
            assert_eq!(
                CellId::from_post_order_index(post).unwrap(),
                cell,
                "post round trip {identifier}"
            );
        }

        let total = resolution_offset(MAX_RESOLUTION + 1);
        assert_eq!(total, 1_389_765_141_638_880);
        assert_eq!(
            CellId::from_level_order_index(total),
            Err(Error::InvalidCellIndex {
                index: total,
                order: "level"
            })
        );
        assert_eq!(
            CellId::from_post_order_index(total),
            Err(Error::InvalidCellIndex {
                index: total,
                order: "post"
            })
        );
    }

    #[test]
    fn traversal_indices_round_trip_every_cell_through_resolution_four() {
        for face_number in 0..6 {
            let root = CellId::new(Face::from_number(face_number).unwrap(), Vec::new()).unwrap();
            for resolution in 0..=4 {
                for cell in root.descendants(resolution).unwrap() {
                    assert_eq!(
                        CellId::from_level_order_index(cell.level_order_index()).unwrap(),
                        cell
                    );
                    assert_eq!(
                        CellId::from_post_order_index(cell.post_order_index()).unwrap(),
                        cell
                    );
                }
            }
        }
    }

    #[test]
    fn cell_order_is_upstream_post_order() {
        let mut cells = ["N", "N0", "N00", "N01", "N08", "N1", "O0"]
            .map(|identifier| identifier.parse::<CellId>().unwrap());
        cells.sort();
        assert_eq!(
            cells.each_ref().map(ToString::to_string),
            ["N00", "N01", "N08", "N0", "N1", "N", "O0"]
        );
        for cells in cells.windows(2) {
            assert!(cells[0].post_order_index() < cells[1].post_order_index());
        }
    }

    #[test]
    fn predecessor_and_successor_match_upstream_examples() {
        let cell: CellId = "N82".parse().unwrap();
        assert_eq!(cell.successor().unwrap().to_string(), "N83");
        assert_eq!(cell.successor_at(0).unwrap().unwrap().to_string(), "O");
        assert_eq!(cell.successor_at(1).unwrap().unwrap().to_string(), "O0");
        assert_eq!(cell.successor_at(3).unwrap().unwrap().to_string(), "N830");

        let cell: CellId = "N08".parse().unwrap();
        assert_eq!(cell.predecessor().unwrap().to_string(), "N07");
        assert_eq!(cell.predecessor_at(0).unwrap(), None);
        assert_eq!(cell.predecessor_at(1).unwrap(), None);
        assert_eq!(cell.predecessor_at(3).unwrap().unwrap().to_string(), "N088");

        let terminal: CellId = "S".parse().unwrap();
        assert_eq!(terminal.successor(), None);
        assert_eq!(terminal.successor_at(15).unwrap(), None);
        assert_eq!(terminal.predecessor().unwrap().to_string(), "R");
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
    fn region_shape_and_rotation_match_upstream() {
        let cases = [
            ("N", Region::NorthPolar, CellShape::Cap),
            ("S", Region::SouthPolar, CellShape::Cap),
            ("P2", Region::Equatorial, CellShape::Quad),
            ("N4", Region::NorthPolar, CellShape::Cap),
            ("N0", Region::NorthPolar, CellShape::Dart),
            ("N43", Region::NorthPolar, CellShape::SkewQuad),
            ("N404", Region::NorthPolar, CellShape::Dart),
            ("N246", Region::NorthPolar, CellShape::Dart),
        ];
        for (identifier, region, shape) in cases {
            let cell: CellId = identifier.parse().unwrap();
            assert_eq!(cell.region(), region, "{identifier}");
            assert_eq!(cell.shape(), shape, "{identifier}");
        }

        let cell: CellId = "N0".parse().unwrap();
        assert_eq!(
            (0..4)
                .map(|turns| cell.rotated(turns).to_string())
                .collect::<Vec<_>>(),
            ["N0", "N2", "N8", "N6"]
        );
    }

    #[test]
    fn ellipsoidal_direction_names_round_trip() {
        for name in [
            "north",
            "south",
            "east",
            "west",
            "north_east",
            "north_west",
            "south_east",
            "south_west",
            "north_0",
            "north_3",
            "south_0",
            "south_3",
        ] {
            let direction: EllipsoidalDirection = name.parse().unwrap();
            assert_eq!(direction.to_string(), name);
        }
        assert!("left".parse::<EllipsoidalDirection>().is_err());
        assert!("north_4".parse::<EllipsoidalDirection>().is_err());
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
