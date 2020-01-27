renderer
-----------------

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