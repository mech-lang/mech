"""Bootstrap-boundary corruption fixtures, never native execution."""
import unittest
import bootstrap_reuse as reuse


def report():return dict(status='stopped-unqualified',active=None,proposal_sha256=reuse.PRIOR_SHA,attempts=[],
    full_performance_runs=0,chart_admission=False,never_chart_eligible=True,no_automatic_retry=True,
    error_type='AssertionError',error="('Owned command failed', 'build', 1)")


class BootstrapReuse(unittest.TestCase):
    def test_exact_no_native_records_and_failed_first_build(self):reuse.validate_report(report())
    def test_partial_record_or_changed_failure_cannot_reuse_bootstrap(self):
        for key,value in (('attempts',[{}]),('profiles',[{}]),('active',{}),('proposal_sha256','wrong'),
            ('error','different failure'),('error_type','RuntimeError'),('full_performance_runs',1),('chart_admission',True)):
            candidate=report();candidate[key]=value
            with self.assertRaises(AssertionError):reuse.validate_report(candidate)
    def test_only_four_scored_strlen_pointer_casts_and_profiles_change(self):reuse.validate_sources()


if __name__=='__main__':unittest.main(verbosity=2)
