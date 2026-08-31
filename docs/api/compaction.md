# Compaction

Compaction replaces complete groups of nine siblings with their parent. It is
lossless with respect to the represented hierarchy.

## `compact_cells`

```python
compact_cells(cells: Sequence[str]) -> list[str]
```

Recursively compact complete sibling groups, remove duplicates, and return
canonical deterministic order.

```python
children = rh.cell_to_children("Q4")
assert rh.compact_cells(children) == ["Q4"]
```

## `uncompact_cells`

```python
uncompact_cells(cells: Sequence[str], resolution: int) -> list[str]
```

Expand every input cell to a common target resolution. An input cell deeper
than the target is invalid.

```python
assert rh.uncompact_cells(["Q4"], 2) == rh.cell_to_children("Q4")
```

Use compact form for storage, transfer, and set operations. Use uncompact form
for uniform-resolution map rendering and numeric aggregation.

