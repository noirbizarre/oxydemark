# API reference

OxydeMark exposes two public surfaces, both frozen and versioned in lockstep
under the policy defined in [OMEP-0008](../specs/OMEP-0008-public-api.md).

<div class="grid cards" markdown>

- **[Python API](python.md)**

    The `oxydemark` package: the engine, the plugin protocol, the AST types and
    the parsing/rendering functions.

- **[Rust API](rust.md)**

    The `oxydemark` crate root, for downstream crates linking the `rlib`
    without PyO3.

</div>

## Stability

| Surface | Tier | Guarantee |
| --- | --- | --- |
| `oxydemark.__all__` | stable | Breaking changes bump the MINOR version while 0.x. |
| `oxydemark` crate root | stable | Same policy, versioned in lockstep with the Python package. |
| `oxydemark.contrib` | provisional | Public and documented, but may change or be removed in a MINOR release. |
| `oxydemark._core` | internal | Must not be imported directly by consumers. |
