bf
-------------------
Binary format is an optimized format for storing various game files after importing.

### Conventions

Integers are little-endian.

#### Header
All files contain header consisting of: 
- magic string 'BF' (u16)
- kind number (u8)
- version (u8)
- *padding* (u32)
- kind dependant data (u64)
- compressed size (u64)
- uncompressed size (u64)

If the file is not compressed then the `compressed size` is equal to `0`. 

Right after the header comes the payload (either LZ4 compressed or not). 
Payload data structure depends on the type of file.

#### Kinds
Following constants are valid kind discriminator values:

```
Image = 0
Geometry/Model = 1
Audio = 2
Material = 3
VirtualFileSystem = 4
CompiledShader = 5
Scene = 6 
```
### Image

Formats: DXT1, DXT3, DXT5, RGB8, RGBA8, (and their srgb variants)

The following values are stored inside the `kind additional data` field of header.
- width (u16)
- height (u16)
- format (u8)
- *padding* (u8)
- *padding* (u16)

The payload contains all mip-maps in the width decreasing order. It is possible to
seek directly to the n-th mip-map by computing the size of preceding mip-maps using the width,
height and format.

### Model / Geometry

Each geometry contains multiple lists.

The following values are stored inside the `kind additional data` field of header.
- *nothing*

Compressed payload consists of header containing information about the payload which follows the header.

Geometry header:
- global flags (u32)
- num of lists (u32)
- lists header
  - list type (u16)
  - list flags (u16)
  - list length (u32)
- payload (lists data)

Lists are encoded in payload in the same order as they are specified in the header. It is possible to seek to
required list by reading the geometry lists header list.

Allowed list types:

```
Positions = 0 (float3)
Normals = 1 (float3)
Tangents = 2 (float3)
Colors = 3 (float3)
UV1 = 4 (float2)
UV2 = 5 (float2)
UV3 = 6 (float2)
UV4 = 7 (float2)
Indices(u8) = 8 (u8)
Indices(u16) = 9 (u16)
Indices(u32) = 10 (u32)
```


### Virtual File System

VFS files consist of two parts: header(s) and data. They are generally 
uncompressed because their content (individual files) are compressed.

VFS Header consists of:
- number of entries
- entry(ies)
  - name (null terminated utf8 string)
  - length (u32)
  - pointer to start of file (u32)

Right after the header the data part comes.
