# Rust quickstart

The `rhealpixdggs` crate is the dependency-light core. It contains no Python,
GEOS, Shapely, Arrow, or file-format types.

From another workspace crate, add a path dependency:

```toml
[dependencies]
rhealpixdggs = { path = "../rhealpixdggs-rs/crates/rhealpixdggs" }
```

Index a coordinate and inspect the result:

```rust
use rhealpixdggs::{Result, RhealpixDggs};

fn main() -> Result<()> {
    let dggs = RhealpixDggs::wgs84_003();
    let cell = dggs.cell_from_lonlat(175.611, -40.356, 8)?;

    assert_eq!(cell.to_string(), "R88756047");
    assert_eq!(cell.resolution(), 8);
    assert_eq!(cell.parent().unwrap().to_string(), "R8875604");

    let boundary = dggs.cell_boundary_lonlat(&cell, 8, false)?;
    let neighbors = dggs.ellipsoidal_neighbors(&cell)?;
    println!("{} points, {} neighbours", boundary.len(), neighbors.len());
    Ok(())
}
```

!!! warning "Rust coordinate order"
    Rust geographic methods use `(longitude, latitude)` in degrees. This is the
    opposite of the H3-style Python facade.

For the complete surface, see the [Rust API reference](../api/rust.md) or run:

```bash
cargo doc -p rhealpixdggs --no-deps --open
```

