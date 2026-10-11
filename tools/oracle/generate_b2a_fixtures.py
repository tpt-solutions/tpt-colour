"""Generates LittleCMS reference codes for the sRGB -> profile (B2A) direction.

Usage: python generate_b2a_fixtures.py <profile.icc> <out_dir> <prefix>
Requires Pillow, which bundles LittleCMS 2.x. Writes <prefix>_relative.txt and
<prefix>_perceptual.txt, one 'r g b -> r g b' line per grid sample, with 8-bit codes.
"""

import sys
from PIL import Image, ImageCms

LEVELS = [0, 51, 102, 153, 204, 255]


def main(profile_path, out_dir, prefix):
    srgb = ImageCms.createProfile("sRGB")
    dst = ImageCms.ImageCmsProfile(profile_path)
    samples = [(r, g, b) for r in LEVELS for g in LEVELS for b in LEVELS]
    image = Image.new("RGB", (len(samples), 1))
    image.putdata(samples)
    for name, intent in (("relative", 1), ("perceptual", 0)):
        transform = ImageCms.buildTransformFromOpenProfiles(
            srgb, dst, "RGB", "RGB", renderingIntent=intent
        )
        out = ImageCms.applyTransform(image, transform)
        lines = [
            f"# LittleCMS via Pillow, sRGB -> {profile_path.split('/')[-1]} ({name}), B2A direction",
        ]
        for (src, dev) in zip(samples, out.getdata()):
            lines.append(f"{src[0]} {src[1]} {src[2]} -> {dev[0]} {dev[1]} {dev[2]}")
        with open(f"{out_dir}/{prefix}_{name}.txt", "w", encoding="utf-8") as f:
            f.write("\n".join(lines) + "\n")


if __name__ == "__main__":
    main(*sys.argv[1:4])
