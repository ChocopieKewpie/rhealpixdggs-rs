---
title: Cell identifiers
description: Understand canonical cell strings and the stable resolution-major u64 mapping.
---

A cell has two losslessly interchangeable public forms:

- a human-readable hierarchy string such as `Q381`;
- a deterministic unsigned 64-bit integer such as `3049`.

## Canonical strings

The first character is one of the six face letters. Each following character
is an aperture-9 child digit from `0` through `8`. No separators, padding, or
lowercase variants are accepted by `is_valid_cell`.

## Stable integer mapping

The current representation is resolution-major—not bit-packed. For resolution
(r):

\[
\operatorname{id} = \operatorname{offset}(r)
  + \operatorname{face} \times 9^r
  + \operatorname{base9}(\text{digits})
\]

\[
\operatorname{offset}(r) = 6 \times \frac{9^r - 1}{8}
\]

Face numbers are `N=0`, `O=1`, `P=2`, `Q=3`, `R=4`, and `S=5`.

![Worked Q381 to integer mapping](/rhealpixdggs-rs/images/stable-u64.svg)

```python
integer = rh.str_to_int("Q381")
assert integer == 3049
assert rh.int_to_str(integer) == "Q381"
```

All currently supported cells through resolution 15 fit in 51 bits. The public
type remains `u64` so storage and language bindings have a stable unsigned
integer contract.

## Level order versus post order

The stable integer representation is the level-order index. The separate
post-order index controls canonical tree traversal and comparison, where
descendants appear before parents. Convert explicitly:

```python
level = rh.cell_to_level_order_index("Q381")
post = rh.cell_to_post_order_index("Q381")

assert rh.level_order_index_to_cell(level) == "Q381"
assert rh.post_order_index_to_cell(post) == "Q381"
```

Do not store post-order indices as interchange IDs unless that ordering is the
specific feature you need.
