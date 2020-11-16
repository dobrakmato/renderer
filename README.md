renderer
-----------------
![build status](https://github.com/dobrakmato/renderer/workflows/Rust/badge.svg)

![apple an ice](https://i.imgur.com/xWoAjcn.png)

### Prerequisites

You need rust, cargo with stable toolchain. We also include [shaderc](https://github.com/google/shaderc-rs) so if
no prebuild binary is available for you system you will need to install dependencies required by that crate. 

### Modules
This project contains following modules.

- [core](core/README.md) - library with code used in other crates
- [bf](bf/README.md) - library for working with bf files (based on [bincode](https://github.com/servo/bincode))
- [bfinfo](bfinfo/README.md) - app to introspect / extract metadata from bf files
- [img2bf](img2bf/README.md) - app to convert image data from conventional image formats to bf file
- [obj2bf](obj2bf/README.md) - app to convert mesh data from conventional mesh formats to bf file
- [matcomp](matcomp/README.md) - app to create material files from command line
- [renderer](renderer/README.md) - simple vulkan-based renderer
