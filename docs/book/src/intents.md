# Rendering intents

An ICC transform has to decide what to do with colours the destination cannot show, and how
closely to match the source's white. The rendering intent chooses that. tpt-colour supports the
four ICC intents:

| Intent | Out-of-gamut colours | Black point compensation | Typical use |
|---|---|---|---|
| `Perceptual` | Compressed toward the gamut boundary, keeping lightness and hue | On for v4 destinations | Photographs, mixed content |
| `RelativeColorimetric` | Clipped | Off, unless you ask for it | Logos, brand colours, proofs |
| `Saturation` | Clipped | On for v4 destinations | Charts and graphics where vividness matters |
| `AbsoluteColorimetric` | Clipped | Never | Simulating paper white, proofing |

Only `Perceptual` gamut-maps. The other three intents clip to the destination gamut, so a value
outside it is not changed to suit the rest of the colour.

## Choosing an intent

- Use `Perceptual` for pictures where the overall look matters more than any single colour.
- Use `RelativeColorimetric` when a colour must stay exact where it is in gamut, and clipping the
  rest is acceptable.
- Use `AbsoluteColorimetric` only when the white point itself must be kept, for example when
  simulating a print on a particular paper.

The intent is an argument to `ColourManager::transform`, and it is cached separately for each
intent. The intent names parse with `FromStr`: `"perceptual"`, `"relative"`, `"saturation"` and
`"absolute"`.

## Absolute colorimetric

Absolute colorimetric scales XYZ by the ratio of the source and destination media white points,
as the ICC specification asks. It uses the profiles' `wtpt` tags. Adaptation (`chad`) is not used
for this, because the colorants are already D50-adapted.
