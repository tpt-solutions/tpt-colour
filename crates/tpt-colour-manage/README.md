# tpt-colour-manage

Profile management and gamut work on top of `tpt-colour-icc`.

- `ColourManager`: registers profiles by name and caches the transforms built from them.
- Gamut boundaries for sRGB, Display P3, Rec. 2020 and matrix/TRC ICC profiles.
- Perceptual gamut mapping by OKLCH chroma reduction, following CSS Color 4.
- HDR helpers: PQ (ST 2084), HLG (BT.2100) and extended Reinhard tone mapping.

Part of the [tpt-colour](https://github.com/tpt-solutions/tpt-colour) workspace.
