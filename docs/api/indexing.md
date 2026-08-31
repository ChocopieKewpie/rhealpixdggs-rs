# Indexing

Indexing maps geographic points to cells and cell nuclei or centroids back to
geographic coordinates.

## `latlng_to_cell`

```python
latlng_to_cell(latitude: float, longitude: float, resolution: int) -> str
```

Return the canonical WGS84_003 cell containing a coordinate.

| Parameter | Description |
| --- | --- |
| `latitude` | Latitude in `[-90, 90]` degrees. |
| `longitude` | Longitude in degrees; canonical geographic inputs use `[-180, 180]`. |
| `resolution` | Target resolution from 0 through 15. |

```python
cell = rh.latlng_to_cell(-40.356, 175.611, 8)
assert cell == "R88756047"
```

## `latlngs_to_cells`

```python
latlngs_to_cells(
    coordinates: Sequence[tuple[float, float]],
    resolution: int,
) -> list[str]
```

Index many `(latitude, longitude)` pairs and preserve input order. This scalar
collection function returns strings. For large numeric arrays, prefer the
[NumPy bulk function](numpy.md#latlngs_to_cells), which returns `uint64` IDs
and avoids per-cell Python objects.

## `cell_to_latlng`

```python
cell_to_latlng(cell: str) -> tuple[float, float]
```

Return the inverse-projected planar nucleus as `(latitude, longitude)`.

## `cell_to_centroid`

```python
cell_to_centroid(cell: str) -> tuple[float, float]
```

Return the geographic/ellipsoidal centroid as `(latitude, longitude)`. This is
the point tested by centroid polygon coverage.

## `is_valid_cell`

```python
is_valid_cell(cell: str) -> bool
```

Return `True` only for a canonical face letter followed by zero to fifteen
digits in the range `0`–`8`.

```python
assert rh.is_valid_cell("Q381")
assert not rh.is_valid_cell("q381")
assert not rh.is_valid_cell("Q39")
```

