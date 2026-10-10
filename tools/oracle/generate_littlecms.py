"""Generates LittleCMS reference fixtures for crates/tpt-colour-oracle.

Requires Pillow (which bundles LittleCMS 2). Profiles are read from a directory given as
the first argument, and are NOT copied into the repository. Only the sampled outputs are
written, one line per sample, as integers in 0..=255.

    python tools/oracle/generate_littlecms.py "C:/Windows/System32/spool/drivers/color"
"""
import itertools
import pathlib
import sys

from PIL import Image, ImageCms

OUT = pathlib.Path(__file__).resolve().parents[2] / "crates/tpt-colour-oracle/tests/fixtures"
LEVELS = [0, 64, 128, 191, 255]


def transform(src_profile, dst_profile, in_mode, samples):
    """Runs LittleCMS on 8-bit samples; returns RGB triples."""
    t = ImageCms.buildTransform(src_profile, dst_profile, in_mode, "RGB", ImageCms.Intent.PERCEPTUAL)
    img = Image.new(in_mode, (len(samples), 1))
    img.putdata([tuple(s) for s in samples])
    out = ImageCms.applyTransform(img, t)
    return list(out.getdata())


def write(name, lines, header):
    OUT.mkdir(parents=True, exist_ok=True)
    path = OUT / name
    with path.open("w", encoding="utf-8") as f:
        f.write("# " + header + "\n")
        for line in lines:
            f.write(line + "\n")
    print(f"wrote {path} ({len(lines)} samples)")


def main():
    colour_dir = pathlib.Path(sys.argv[1])
    version = ImageCms.core.littlecms_version
    header = f"LittleCMS {version} via Pillow, perceptual intent, no BPC"

    # CMYK (SWOP) → sRGB: exercises the LUT (mAB/mft) path.
    swop = ImageCms.ImageCmsProfile(str(colour_dir / "RSWOP.icm"))
    srgb = ImageCms.createProfile("sRGB")
    cmyk = [list(c) for c in itertools.product(LEVELS, repeat=4)]
    rgb = transform(swop, srgb, "CMYK", cmyk)
    write(
        "rswop_cmyk_to_srgb.txt",
        [" ".join(map(str, c)) + " -> " + " ".join(map(str, o)) for c, o in zip(cmyk, rgb)],
        header + ", input CMYK RSWOP.icm",
    )

    # Gamma 1.8 RGB → sRGB: exercises the matrix/TRC path with a non-sRGB curve.
    g18 = ImageCms.ImageCmsProfile(str(colour_dir / "ewrgb18.icm"))
    rgb_in = [list(c) for c in itertools.product(LEVELS, repeat=3)]
    rgb_out = transform(g18, srgb, "RGB", rgb_in)
    write(
        "ewrgb18_to_srgb.txt",
        [" ".join(map(str, c)) + " -> " + " ".join(map(str, o)) for c, o in zip(rgb_in, rgb_out)],
        header + ", input RGB ewrgb18.icm (gamma 1.8)",
    )


if __name__ == "__main__":
    main()
