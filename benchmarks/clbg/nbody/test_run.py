import unittest
from run import endpoint_summary, energies, strip_comments, validate

class CorrectnessGuardTests(unittest.TestCase):
    def test_nonfinite_output_is_rejected(self):
        for value in ("nan", "inf", "-inf"):
            with self.assertRaises(ValueError):
                energies(f"-0.169075164\n{value}\n")

    def test_wrong_energy_and_extra_output_are_rejected(self):
        for output in ("-0.169075164\n-0.169000000\n", "log\n-0.169075164\n-0.169087605\n"):
            with self.assertRaises(ValueError):
                validate(output, [-0.169075164, -0.169087605])

    def test_precision_tolerance(self):
        validate("-0.169075164\n-0.169087604\n", [-0.169075164, -0.169087605])

class EndpointTests(unittest.TestCase):
    def test_fastest_and_smallest_are_actual_programs(self):
        rows = [{"id": name, "language": "example", "median_seconds": time,
                 "source": {"gzip1_bytes": size}}
                for name, time, size in (("fast", 1, 200), ("small", 3, 100), ("dominated", 5, 300))]
        e = endpoint_summary(rows)["languages"]["example"]
        self.assertEqual((e["fastest"], e["smallest"]), ("fast", "small"))
        self.assertEqual(e["smallest_time_over_fastest"], 3)
        self.assertEqual(e["fastest_size_over_smallest"], 2)
        self.assertFalse(e["same_entry"])

    def test_same_entry_preserves_candidate_count(self):
        rows = [{"id": "one", "language": "example", "median_seconds": 1,
                 "source": {"gzip1_bytes": 100}}]
        e = endpoint_summary(rows)["languages"]["example"]
        self.assertEqual(e["candidate_count"], 1)
        self.assertTrue(e["same_entry"])

class CommentTests(unittest.TestCase):
    def test_comment_like_text_in_strings_survives(self):
        self.assertEqual(strip_comments('x="http://host/*x*/"; // note\ny=2', "c"), 'x="http://host/*x*/"; \ny=2')
        self.assertEqual(strip_comments('x="# literal" # comment\ny=2', "julia"), 'x="# literal" \ny=2')

if __name__ == "__main__":
    unittest.main()
