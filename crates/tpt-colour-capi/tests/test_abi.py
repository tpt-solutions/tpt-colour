"""Calls the built C library through ctypes, using the signatures in include/tpt_colour.h.

Build the library first (cargo build --release in crates/tpt-colour-capi), then run:
    python -m unittest discover -s crates/tpt-colour-capi/tests
The library path can be overridden with TPT_COLOUR_LIB.
"""

import ctypes
import os
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
CRATE = HERE.parent


def library_path() -> Path:
    override = os.environ.get("TPT_COLOUR_LIB")
    if override:
        return Path(override)
    name = {"win32": "tpt_colour.dll", "darwin": "libtpt_colour.dylib"}.get(
        sys.platform, "libtpt_colour.so"
    )
    return CRATE / "target" / "release" / name


lib = ctypes.CDLL(str(library_path()))

FloatPtr = ctypes.POINTER(ctypes.c_float)
BytePtr = ctypes.POINTER(ctypes.c_uint8)

lib.tpt_version.restype = ctypes.c_char_p
lib.tpt_parse_colour.argtypes = [ctypes.c_char_p, ctypes.c_size_t, BytePtr]
lib.tpt_parse_colour.restype = ctypes.c_int32
lib.tpt_wcag_contrast.argtypes = [FloatPtr, FloatPtr, FloatPtr]
lib.tpt_wcag_contrast.restype = ctypes.c_int32
lib.tpt_apca_contrast.argtypes = [FloatPtr, FloatPtr, FloatPtr]
lib.tpt_apca_contrast.restype = ctypes.c_int32
lib.tpt_simulate_deficiency.argtypes = [FloatPtr, ctypes.c_uint32, FloatPtr]
lib.tpt_simulate_deficiency.restype = ctypes.c_int32
lib.tpt_manager_new.argtypes = []
lib.tpt_manager_new.restype = ctypes.c_void_p
lib.tpt_manager_free.argtypes = [ctypes.c_void_p]
lib.tpt_manager_free.restype = None
lib.tpt_manager_transform_rgb.argtypes = [
    ctypes.c_void_p,
    ctypes.c_char_p,
    ctypes.c_size_t,
    ctypes.c_char_p,
    ctypes.c_size_t,
    ctypes.c_uint32,
    FloatPtr,
    FloatPtr,
]
lib.tpt_manager_transform_rgb.restype = ctypes.c_int32

OK, NULL_POINTER, INVALID_INPUT, CONVERSION_FAILED = 0, -1, -2, -3


def floats(values):
    return (ctypes.c_float * len(values))(*values)


class VersionTests(unittest.TestCase):
    def test_version_is_c_string(self):
        self.assertEqual(lib.tpt_version(), b"0.1.0")


class ParseTests(unittest.TestCase):
    def parse(self, text: bytes):
        out = (ctypes.c_uint8 * 4)()
        code = lib.tpt_parse_colour(text, len(text), out)
        return code, list(out)

    def test_parses_css(self):
        self.assertEqual(self.parse(b"rebeccapurple"), (OK, [102, 51, 153, 255]))

    def test_unknown_is_invalid_input(self):
        self.assertEqual(self.parse(b"nope")[0], INVALID_INPUT)

    def test_non_utf8_is_invalid_input(self):
        self.assertEqual(self.parse(b"\xff\xfe")[0], INVALID_INPUT)

    def test_null_pointer(self):
        out = (ctypes.c_uint8 * 4)()
        self.assertEqual(lib.tpt_parse_colour(None, 0, out), NULL_POINTER)


class ContrastTests(unittest.TestCase):
    def test_wcag_black_on_white(self):
        out = (ctypes.c_float * 1)()
        code = lib.tpt_wcag_contrast(floats([0, 0, 0]), floats([1, 1, 1]), out)
        self.assertEqual(code, OK)
        self.assertAlmostEqual(out[0], 21.0, places=2)

    def test_apca_black_on_white(self):
        out = (ctypes.c_float * 1)()
        self.assertEqual(lib.tpt_apca_contrast(floats([0, 0, 0]), floats([1, 1, 1]), out), OK)
        self.assertAlmostEqual(out[0], 106.0, delta=0.5)


class VisionTests(unittest.TestCase):
    def test_greys_are_preserved(self):
        out = (ctypes.c_float * 3)()
        code = lib.tpt_simulate_deficiency(floats([0.5, 0.5, 0.5]), 2, out)
        self.assertEqual(code, OK)
        self.assertTrue(all(abs(v - 0.5) < 1e-3 for v in out))

    def test_bad_kind(self):
        out = (ctypes.c_float * 3)()
        self.assertEqual(lib.tpt_simulate_deficiency(floats([0.5] * 3), 9, out), INVALID_INPUT)


class ManagerTests(unittest.TestCase):
    def test_round_trip_and_free(self):
        manager = lib.tpt_manager_new()
        self.assertTrue(manager)
        src, dst = b"srgb", b"display-p3"
        original = [0.8, 0.2, 0.1]
        p3 = (ctypes.c_float * 3)()
        back = (ctypes.c_float * 3)()
        try:
            self.assertEqual(
                lib.tpt_manager_transform_rgb(
                    manager, src, len(src), dst, len(dst), 0, floats(original), p3
                ),
                OK,
            )
            self.assertEqual(
                lib.tpt_manager_transform_rgb(
                    manager, dst, len(dst), src, len(src), 0, p3, back
                ),
                OK,
            )
        finally:
            lib.tpt_manager_free(manager)
        for a, b in zip(back, original):
            self.assertLess(abs(a - b), 2e-3)

    def test_unknown_profile_is_conversion_failure(self):
        manager = lib.tpt_manager_new()
        name = b"no-such-profile"
        out = (ctypes.c_float * 3)()
        try:
            code = lib.tpt_manager_transform_rgb(
                manager, name, len(name), b"srgb", 4, 0, floats([0.1] * 3), out
            )
        finally:
            lib.tpt_manager_free(manager)
        self.assertEqual(code, CONVERSION_FAILED)

    def test_bad_intent(self):
        manager = lib.tpt_manager_new()
        src = b"srgb"
        out = (ctypes.c_float * 3)()
        try:
            code = lib.tpt_manager_transform_rgb(
                manager, src, 4, src, 4, 9, floats([0.1] * 3), out
            )
        finally:
            lib.tpt_manager_free(manager)
        self.assertEqual(code, INVALID_INPUT)

    def test_free_null_is_a_no_op(self):
        lib.tpt_manager_free(None)


if __name__ == "__main__":
    unittest.main()
