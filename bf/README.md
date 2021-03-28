bf
-------------------
Binary format is an optimized format for storing various game 
files after importing. Currently there is no defined format. However
this implementation can be used as a reference.

For serialization to binary data `bincode` crate is used together
with `serde` and for compression purposes `lz4` compression is used.

Compressed structs are wrapped in `Compressed` new-type struct whose
`Serialize` and `Deserialize` traits are implemented in a way that it
recursively calls `bincode`'s `serialize()` and compresses bytes
returned by the call with `lz4`. These structs are currently not
zero-copy.

##### Conventions

Integers are little-endian.

Same serialization rules apply as those written [here](https://github.com/servo/bincode).

### File Header

Each file has a header which contains magic string `BF` and a version number. After that
the file data continues either in LZ4-compressed of uncompressed form.

Currently these file types are supported:
- Image
- Geometry

#### Image

Image width and height are `u16`.

These formats are supported: 
```rust
pub enum Format {
    Dxt1,
    Dxt3,
    Dxt5,
    Rgb8,
    Rgba8,
    SrgbDxt1,
    SrgbDxt3,
    SrgbDxt5,
    Srgb8,
    Srgb8A8,
}
```

#### Geometry

Vertex data is stored in indexed form. Indices are either `u8`, `u16` or `u32`.

These vertex data formats are supported:

```rust
pub enum VertexDataFormat {
    PositionNormalUv, // vec3 (pos), vec3(nor), vec2(uv)
}
```

### Scene / Tree

Each tree has one root node.

Each node may have zero or more children. Each node may have zero or more component attached to it.


These components are supported:
```rust
pub enum Component {
    Name,
    Sky,
    Transform,
    MeshRenderer,
    DirectionalLight,
}
```