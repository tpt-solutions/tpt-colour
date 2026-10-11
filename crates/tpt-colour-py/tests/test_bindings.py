"""Run after installing the wheel: python -m unittest discover -s tests"""

import unittest

import tpt_colour as tc


class ParseTests(unittest.TestCase):
    def test_css_colours(self):
        self.assertEqual(tc.parse_colour("rebeccapurple"), [102, 51, 153, 255])
        self.assertEqual(tc.parse_colour("#f008"), [255, 0, 0, 136])

    def test_rejects_unknown_input(self):
        with self.assertRaises(ValueError):
            tc.parse_colour("not a colour")


class ContrastTests(unittest.TestCase):
    def test_reference_values(self):
        black, white = [0.0, 0.0, 0.0], [1.0, 1.0, 1.0]
        self.assertAlmostEqual(tc.wcag_contrast(black, white), 21.0, places=2)
        self.assertAlmostEqual(tc.apca_contrast(black, white), 106.0, delta=0.5)

    def test_wrong_length_raises(self):
        with self.assertRaises(ValueError):
            tc.wcag_contrast([0.0, 0.0], [1.0, 1.0, 1.0])


class VisionAndPaletteTests(unittest.TestCase):
    def test_greys_survive_simulation(self):
        out = tc.simulate_deficiency([0.5, 0.5, 0.5], "deuteranopia")
        self.assertTrue(all(abs(v - 0.5) < 1e-3 for v in out))

    def test_unknown_deficiency(self):
        with self.assertRaises(ValueError):
            tc.simulate_deficiency([0.5, 0.5, 0.5], "monochromacy")

    def test_harmony_shapes(self):
        self.assertEqual(len(tc.harmony([1.0, 0.0, 0.0], "complement")), 1)
        self.assertEqual(len(tc.harmony([1.0, 0.0, 0.0], "triadic")), 2)


class ManagerTests(unittest.TestCase):
    def test_round_trip_between_builtin_profiles(self):
        m = tc.Manager()
        p3 = m.transform_rgb("srgb", "display-p3", "perceptual", [0.8, 0.2, 0.1])
        back = m.transform_rgb("display-p3", "srgb", "perceptual", p3)
        self.assertTrue(all(abs(a - b) < 2e-3 for a, b in zip(back, [0.8, 0.2, 0.1])))
        self.assertGreaterEqual(m.cached_transforms, 1)

    def test_bad_intent(self):
        with self.assertRaises(ValueError):
            tc.Manager().transform_rgb("srgb", "srgb", "vivid", [0.1, 0.2, 0.3])


if __name__ == "__main__":
    unittest.main()
