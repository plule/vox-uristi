---
title: "Tips and troubleshooting"
layout: single
permalink: /tips
---

## Fine-tuning the render

Vox Uristi exports the data in the `.vox` file with a hierarchy, to allow
further customization when rendering pictures.

![hierarchy](/vox-uristi/assets/tips/hierarchy.png)

Navigating to the "Layers" in MagicaVoxel, as seen on the left side, it's
possible to hide and show various features of the export. For example, hiding
the spatters and the roughness will lead to a less cluttered look.

The "hidden" layer represent all the black tiles that are not discovered by the
player. Hiding this layer can be useful to render a multi-level underground
fortress, while showing it gives a look closer to what's visible in game.

On the right side, the "Outline" can be used to hide part of the export. Each
individual Z-Level can be hidden there, to show the interior of buildings for
example. Then, for each level, it's possible to navigate to hide individual
buildings, or specific map block (16x16 chunk of map), or specific features
by block.

## The corners of the map are cut when trying to render

Vox Uristi exports large models, and MagicaVoxel has a limit in the number of
voxels. This limits can be bumped by checking the option "sparse geometry" in
the sampling settings of the rendering left panel.

## Interior rendering

Even though it is possible to add light sources in MagicaVoxel, the best way to
light a scene is always from the sun and sky, making interior rendering tricky.

The best approach is almost always to hide some upper Z-level to open the roof
and let the sunlight in. In Magica Voxel, use the "Outline" panel on the right
to control which Z-level is visible.

## Vertical Stretching

Dwarf Fortress does not have defined dimensions for tiles, and Vox Uristi
exports them with 3x3x5 voxels. This can be natural for some fortresses, or feel
stretched up in others.

It is possible in MagicaVoxel to make voxels non square, which can help setting
appropriate dimensions: In left panel of the render tab, under "Display
Settings", expand the "Scale" drop-down, and adjust the Z parameter.
