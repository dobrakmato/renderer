![build status](https://github.com/dobrakmato/renderer/workflows/Rust/badge.svg)
-----------------

### Prerequisites

You need rust, cargo with stable toolchain. We also include [shaderc](https://github.com/google/shaderc-rs) so if
no prebuild binary is available for you system you will need to install dependencies required by that crate. 

### Modules
This project contains following modules.

- [bf](bf/README.md) - library for working with bf files
- [bfinfo](bfinfo/README.md) - app to extract metadata from bf files
- [img2bf](img2bf/README.md) - app to convert image data from conventional image format to bf file
- [obj2bf](obj2bf/README.md) - app to convert mesh data from conventional mesh formats to bf file
- [renderer](renderer/README.md) - simple vulkan-based renderer