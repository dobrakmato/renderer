renderer
-----------------

## Architecture

After many tries I decided on separating different parts on the renderer by their memory access patterns. This
should result in clearer design and less annoyance from borrow checker. Parts of the code use `Arc` mainly because
`vulkano` does so. Until the `vulkano` is replaced with something else `Arc`s are here to stay.

### Content

- [x] loading happens in IO thread and does not block rendering
- [x] loading from local disk
- [ ] loading from HTTP
- [ ] caching of HTTP downloaded resources
- [ ] loading of multiple resources at same time
- [ ] pipelined loading
- [ ] resource hot-reloading for local files




Objects:
- Static
- Movable



Input textures:
- albedo (RGB)
- normal (XYZ)
- roughness (R)
- metallic (R)
- occlusion (R)
- height (R)
- emission (RGB)

Shaders:
- PBR
- SkinnedPBR

Variants:
- Basic (albedo+normal+roughness+metallic+occlusion)
- Basic+AlphaCutoff
- Parallax-occlusion (+height)
- Emission (+emission)

Lighting models:
- Opaque
- SubsurfaceScattering
- Hair

G-Buffer:
- [D16] Depth
- [RGB10A2] Normal (XYZ), Lighting Model (A)
- [RGBA32] Albedo (RGB),  Occlusion (A)
- [RGBA32] Metallic (R), Roughness (G)
- [RGBA32] SubsurfaceColor (RGB)

HDRBuffer:
- [B10G11R11] HDR Color

Render Passes:
- Main
  - Geometry
  - Lighting
  - Skybox
  - Tonemap
  - FXAA