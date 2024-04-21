---
title: "☼Vox Uristi☼"
layout: splash
permalink: /
header:
  overlay_filter: 0.5
  overlay_image: /assets/gallery/banner.jpg
---

[<i class='fab fa-windows'></i> Download for Windows]({%include latest-download-windows.html%}){: .btn .btn--success .btn--x-large}
[<i class='fab fa-linux'></i> Download for Linux]({%include latest-download-linux.html%}){: .btn .btn--success .btn--x-large}
[<i class='fas fa-external-link-alt'></i> View on Github](https://github.com/plule/vox-uristi){: .btn .btn--primary .btn--x-large}

**Vox Uristi** exports Dwarf Fortress maps in a voxel format to create beautiful
rendering of your fortresses.

It uses DFHack to read the fortress data and export it in the `.vox` format. The
resulting file can then be opened in a software such as MagicaVoxel to render
it.

| <video autoplay="autoplay" loop="loop" width="500" height="500"><source src="/vox-uristi/assets/gallery/heavenfall/spin.webm" type="video/webm"></video> |
| -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| *Heavenfall, a fortress by Horrigant*                                                                                                                    |

[<i class="fas fa-info"></i> Usage](/vox-uristi/usage){: .btn .btn--primary}
[<i class="fas fa-images"></i> Gallery](/vox-uristi/gallery){: .btn .btn--primary}
[<i class="fas fa-exclamation"></i> Tips](/vox-uristi/tips){: .btn .btn--primary}
[<i class="fas fa-clock"></i> Changelog](/vox-uristi/changelog){: .btn .btn--primary}

## Features/Roadmap

### Next

- ☑ Base building blocks (walls, floors, fortifications)
- ☑ Water, magma and grass
- ☑ Basic material colors
- ☑ Directional ramps
- ☑ Rough/Smooth floor representation
- ☑ Basic tree support, inaccurate but good enough
- ☑ Most construction items (doors, windows, bars, bridges, furnitures, workshops)
- ☑ Flows, waves and mist
- ☑ Seasonal plants
- ☑ Detailed materials (metallic, water, light emission)
- ☑ Building content (books on bookcases, items in workshops, ...)
- ☑ Spatters
- ☑ Export the buildings as individual selectable objects
- ☐ Item state (opened/closed)
- ☐ Rails
- ☐ Split the export into useful layers

### Maybe some day

- ☐ Export some animations
- ☐ Customizable models
- ☐ Support different voxel resolutions
- ☐ Minecraft map format

## Other Dwarf Fortress visualisation tools

Vox Uristi is only intended to make one-off renders of fortresses. Other tools
can be used for different kind of usage or render.

For real-time isometric rendering:
[stonesense](https://docs.dfhack.org/en/stable/docs/tools/stonesense.html).

For real-time 3D rendering: [Armok Vision](https://github.com/RosaryMala/armok-vision).

For one-off render of the world map: [VoxelFortress](https://github.com/RosaryMala/VoxelFortress/releases/tag/v1.0.0).
