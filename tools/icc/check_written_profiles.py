"""Checks profiles written by tpt-colour against LittleCMS (through Pillow).

Run the example first, which writes the profiles and tpt-colour's own results:

    cargo run -p tpt-colour --example write_profile -- target/icc-check

Then run this with the same directory:

    python tools/icc/check_written_profiles.py target/icc-check

For each written profile, LittleCMS converts the same 8-bit test colours from the written sRGB
profile, using the relative colorimetric intent with no black point compensation. The results are
compared with tpt-colour's, and the worst difference in 8-bit codes must be at most TOLERANCE.
"""

import pathlib
import sys

from PIL import Image, ImageCms

TOLERANCE = 1  # 8-bit codes. The parity target for LittleCMS elsewhere in this project.
PROFILES = ["srgb_builtin", "p3_gamma22", "p3_table"]


def read_expectations(path: pathlib.Path):
    rows = {}
    for line in path.read_text(encoding="utf8").splitlines():
        name, *values = line.split()
        rows.setdefault(name, []).append([int(v) for v in values])
    return rows


def littlecms_results(directory: pathlib.Path, name: str, colours):
    src = ImageCms.ImageCmsProfile(str(directory / "srgb_builtin.icc"))
    dst = ImageCms.ImageCmsProfile(str(directory / f"{name}.icc"))
    transform = ImageCms.buildTransformFromOpenProfiles(
        src,
        dst,
        "RGB",
        "RGB",
        renderingIntent=ImageCms.Intent.RELATIVE_COLORIMETRIC,
        flags=0,
    )
    image = Image.new("RGB", (len(colours), 1))
    image.putdata([tuple(c[:3]) for c in colours])
    converted = ImageCms.applyTransform(image, transform)
    return [list(converted.getpixel((x, 0))) for x in range(len(colours))]


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__)
        return 2
    directory = pathlib.Path(sys.argv[1])
    expectations = read_expectations(directory / "tpt_expected.txt")
    print(f"LittleCMS {ImageCms.core.littlecms_version}")

    failed = False
    for name in PROFILES:
        rows = expectations[name]
        inputs = [row[:3] for row in rows]
        ours = [row[3:] for row in rows]
        theirs = littlecms_results(directory, name, inputs)
        worst = max(
            abs(a - b) for mine, other in zip(ours, theirs) for a, b in zip(mine, other)
        )
        status = "ok" if worst <= TOLERANCE else "FAIL"
        print(f"{name:14} worst difference {worst} codes  {status}")
        failed |= worst > TOLERANCE

    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
