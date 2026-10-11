"""Generates double-precision LittleCMS references for crates/tpt-colour-oracle.

The 8-bit references in generate_littlecms.py go through LittleCMS's 8-bit pipeline, which clamps and
quantises intermediate values. On PRMG and ROMM that moves the output by up to 8 codes from LittleCMS's
own double-precision result, so these references use the double-precision path instead.

Requires python-lcms2 (a wheel bundling LittleCMS 2.17, the same version as the 8-bit generator).
Profiles are read from the directory given as the first argument and are NOT copied into the repo.
The sample grid is taken from the existing 8-bit fixture, so both references cover the same inputs.

    python tools/oracle/generate_littlecms_double.py "C:/Users/you/icc-test-profiles"
"""
import pathlib
import sys

import lcms2
import numpy as np

OUT = pathlib.Path(__file__).resolve().parents[2] / "crates/tpt-colour-oracle/tests/fixtures"

# (profile file, intent, grid fixture to reuse the samples from, output name, intent name)
CASES = [
    ("PRMG_RGB-sRGB_based.icc", "RELATIVE_COLORIMETRIC", "prmg_relative_to_srgb.txt", "prmg_relative_dbl_to_srgb.txt"),
    ("ISO22028-2_ROMM-RGB.icc", "RELATIVE_COLORIMETRIC", "romm_relative_to_srgb.txt", "romm_relative_dbl_to_srgb.txt"),
    ("ISO22028-2_ROMM-RGB.icc", "PERCEPTUAL", "romm_perceptual_to_srgb.txt", "romm_perceptual_dbl_to_srgb.txt"),
]


def read_inputs(name):
    lines = [l for l in (OUT / name).read_text(encoding="utf-8").splitlines() if l and not l.startswith("#")]
    return [l.split("->")[0].split() for l in lines]


def main():
    colour_dir = pathlib.Path(sys.argv[1])
    srgb = lcms2.create_profile("sRGB")
    raw = str(lcms2.get_version())
    version = f"{raw[0]}.{raw[1:]}"
    for profile_name, intent, grid, out_name in CASES:
        profile = lcms2.open_profile(str(colour_dir / profile_name))
        codes = read_inputs(grid)
        samples = np.array([[float(c) / 255.0 for c in row] for row in codes])
        transform = lcms2.Transform(profile, "RGB_DBL", srgb, "RGB_DBL", intent=intent, flags="NONE")
        # Values are clamped to the output range, as tpt-colour's output is.
        out = np.clip(np.asarray(transform.apply(samples)), 0.0, 1.0) * 255.0
        header = (f"# LittleCMS {version} double precision, {intent.lower().replace('_', ' ')} intent, "
                  f"no BPC, input RGB {profile_name}")
        lines = [" ".join(row) + " -> " + " ".join(f"{v:.6f}" for v in o) for row, o in zip(codes, out)]
        (OUT / out_name).write_text("\n".join([header] + lines) + "\n", encoding="utf-8")
        print(f"wrote {out_name} ({len(lines)} samples)")


if __name__ == "__main__":
    main()
