"""Additive parent-directory repair; no compiler or native process here."""
from pathlib import Path


def initialize_build_root(build):
    build = Path(build)
    assert not build.exists(), 'Preserve prior failed roots; never retry in place'
    build.mkdir()
    (build / 'bin').mkdir()
    return build


def validate_failed_v7(report, cohort):
    expected = str(Path(cohort) / '.build/private-v7/bin/standard-primitives')
    assert report['status'] == 'stopped-unqualified'
    assert report['proposal_sha256'] == 'd6d318a54b4277527ceb9a2ecd0ab35c46c86ddfb55228e453df5117ac111512'
    assert report['error_type'] == 'FileNotFoundError'
    assert report['error'] == "[Errno 2] No such file or directory: '" + expected + "'"
    assert report['attempts'] == [] and report['active'] is None and not report.get('profiles')
    assert report['reused_bounded_cases'] == 154 and report['reused_cases_relabelled_as_attempts'] is False
    assert report['full_performance_runs'] == 0 and report['chart_admission'] is False
    assert report['no_automatic_retry'] is True

