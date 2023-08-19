---
title: "Usage"
layout: single
permalink: /usage
---

[<i class='fas fa-download'></i> Download for Windows]({%include latest-download-windows.html%}){: .btn .btn--success .btn--x-large}
[<i class='fas fa-download'></i> Download for Linux]({%include latest-download-linux.html%}){: .btn .btn--success .btn--x-large}

⚠ Vox Uristi is in development. The exported map is lacking important features,
and could not work at all in some cases. Please report any issue you see. Save
your game before exporting, as certain bug could trigger a Dwarf Fortress crash
{: .notice--danger }

First, ensure you have [Dwarf
Fortress](https://store.steampowered.com/app/975370/Dwarf_Fortress/) with
[DFHack](https://store.steampowered.com/app/2346660/DFHack__Dwarf_Fortress_Modding_Engine/).
Vox Uristi is mostly tested with the latest Steam release, but could work with
previous versions too.

While in game in the save you wish to export, run Vox Uristi and select the
upper and lower bound to export. Only the zone between these two altitudes will
be included in the exported model. It works best by selecting the surface area
of your map.

![how-to](assets/how-to.gif)

Once exported, open the `.vox` file with [MagicaVoxel](https://ephtracy.github.io/).

⚠ The same website has a dedicated voxel viewer. At the moment, the exported
files are not correctly rendered by this viewer.
{: .notice--danger }

You can see the whole process on this mod spotlight made by Blind.

{% include video id="CDqMuBZsNH0" provider="youtube" %}
