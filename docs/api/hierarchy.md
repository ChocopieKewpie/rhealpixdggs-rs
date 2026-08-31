# Hierarchy and identifiers

## `get_resolution`

```python
get_resolution(cell: str) -> int
```

Return the number of child digits. Root cells have resolution 0.

## `get_base_cell_number`

```python
get_base_cell_number(cell: str) -> int
```

Return the root-face number: `N=0`, `O=1`, `P=2`, `Q=3`, `R=4`, `S=5`.

## `cell_to_parent`

```python
cell_to_parent(cell: str, resolution: int | None = None) -> str | None
```

Without `resolution`, return the direct parent. A root cell returns `None`.
With a resolution, return that ancestor; the requested value cannot exceed the
input cell resolution.

## `cell_to_children`

```python
cell_to_children(cell: str, resolution: int | None = None) -> list[str]
```

Without `resolution`, return the nine direct children in digit order `0`–`8`.
With a deeper resolution, return all descendants in canonical order.

## `cell_to_successor`

```python
cell_to_successor(cell: str, resolution: int | None = None) -> str | None
```

Return the next cell in post-order traversal, optionally constrained to the
requested resolution. Returns `None` after the final cell.

## `cell_to_predecessor`

```python
cell_to_predecessor(cell: str, resolution: int | None = None) -> str | None
```

Return the previous cell in post-order traversal, optionally at a fixed
resolution. Returns `None` before the first cell.

## `str_to_int`

```python
str_to_int(cell: str) -> int
```

Convert a canonical cell string to its stable resolution-major `u64` value.

## `int_to_str`

```python
int_to_str(cell: int) -> str
```

Decode the stable integer representation. Values outside the valid hierarchy
raise `ValueError`.

## `cell_to_level_order_index`

```python
cell_to_level_order_index(cell: str) -> int
```

Return the stable zero-based resolution-major index. This is the same mapping
used by `str_to_int`.

## `level_order_index_to_cell`

```python
level_order_index_to_cell(index: int) -> str
```

Inverse of `cell_to_level_order_index`.

## `cell_to_post_order_index`

```python
cell_to_post_order_index(cell: str) -> int
```

Return the index in the finite post-order hierarchy through resolution 15.
This ordering is distinct from the stable interchange integer.

## `post_order_index_to_cell`

```python
post_order_index_to_cell(index: int) -> str
```

Inverse of `cell_to_post_order_index`.

![Hierarchy and traversal ordering](../images/grid-traversal.svg)

