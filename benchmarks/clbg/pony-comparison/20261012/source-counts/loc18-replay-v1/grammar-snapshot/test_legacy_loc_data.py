"""Focused metadata-admission fixtures, executed only with the common mutex."""
import copy
import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path
from legacy_loc_data import apply_legacy_loc_audit,validate_reviewed_audit

STUDY=Path(__file__).resolve().parents[2]/"loc-normalization/20261012"
sys.path.insert(0,str(STUDY))
from audit import timing_boundary

MUTEX_FD=None
sha=lambda path:hashlib.sha256(Path(path).read_bytes()).hexdigest()


class LegacyLOCAdmissionFixtures(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root=Path(self.temp.name)
        files={};rows=[];sources={};decisions={}
        for index,(code,original,count,seconds) in enumerate((
            ("a=1;b=2;\nc=3;d=4;\n",2,4,1.),
            ("a=1;b=2;c=3;d=4;e=5;\n",1,5,2.),
            ("a=1;\nb=2;\nc=3;\n",3,3,3.)),1):
            path=self.root/f"source{index}.lua";path.write_text(code)
            identity=sha(path);metric_key=identity+":lua";key=f"toy:lua-{index}"
            file=dict(name=path.name,path=str(path),sha256=identity,code=code)
            if index<3:files[identity]=file
            samples=[seconds+.1,seconds,seconds-.1]
            rows.append(dict(key=key,id=f"lua-{index}",benchmark="toy",language="lua",
                original_loc=original,physical_loc=original,normalized_loc=count,
                source_identities=[metric_key],source_files=[dict(path=str(path),sha256=identity)],
                already_audited=False,samples=samples,seconds=seconds,report="report.json",source_report="report.json",
                algorithm=dict(strict=True),runtime_policy="Lua interpreted",command=["lua",str(path)],
                qualification=dict(admitted=True,full_workload=True,full_output_validated=True,
                    report_completed=True,sample_count_matches=True,sample_count=3)))
            sources[metric_key]=dict(sha256=identity)
            decisions[metric_key]=dict(source_sha256=identity,normalized_loc=count)
        self.data=dict(programs={row["key"]:dict(key=row["key"],id=row["id"],benchmark="toy",language="lua",
            loc=row["original_loc"],seconds=row["seconds"],samples=row["samples"],files=[row["source_files"][0]["sha256"]]) for row in rows[:2]},
            files=files,exclusions=[],languages=[dict(key="lua",label="Lua")],
            benchmarks=[dict(key="toy",groups=[dict(language="lua",label="Lua",fast="toy:lua-1",small="toy:lua-2",programs=["toy:lua-1","toy:lua-2"])])],provenance={})
        current=self.root/"current.json";current.write_text(json.dumps(self.data))
        report=self.root/"report.json";report.write_text(json.dumps(rows))
        self.audit=dict(status="reviewed",coverage_complete=True,rule_version="fixture",
            review=dict(complete_measured_pool=True,source_decisions=decisions),
            pending_needed_measurement=[],sources=sources,candidates=rows,
            current_data=dict(path=str(current),sha256=sha(current)),
            current_legacy_projection={key:{field:p.get(field) for field in ("language","benchmark","loc","seconds","samples","files")} for key,p in self.data["programs"].items()},
            report_pins={"report.json":sha(report)})
        self.path=self.root/"audit.json"

    def apply(self):
        self.path.write_text(json.dumps(self.audit))
        return apply_legacy_loc_audit(self.root,self.data,self.path,common_timing_mutex_fd=MUTEX_FD)

    def test_candidate_proposal_cannot_be_admitted(self):
        self.audit["status"]="candidate-audit-awaiting-review"
        with self.assertRaisesRegex(ValueError,"explicit review"):validate_reviewed_audit(self.audit)

    def test_incomplete_pool_or_pending_winner_blocks_admission(self):
        self.audit["coverage_complete"]=False
        with self.assertRaisesRegex(ValueError,"candidate-pool"):validate_reviewed_audit(self.audit)
        self.audit["coverage_complete"]=True
        self.audit["pending_needed_measurement"]=[dict(key="toy:unmeasured")]
        with self.assertRaisesRegex(ValueError,"full measurements"):validate_reviewed_audit(self.audit)

    def test_full_output_qualification_is_required(self):
        self.audit["candidates"][0]["qualification"]["full_output_validated"]=False
        with self.assertRaisesRegex(ValueError,"full-record qualification"):validate_reviewed_audit(self.audit)

    def test_complete_pool_can_choose_previously_unplotted_compact_winner(self):
        before=copy.deepcopy(self.data)
        result=self.apply()
        group=result["benchmarks"][0]["groups"][0]
        self.assertEqual((group["fast"],group["small"]),("toy:lua-1","toy:lua-3"))
        self.assertEqual(result["programs"]["toy:lua-1"]["samples"],before["programs"]["toy:lua-1"]["samples"])
        self.assertEqual(result["programs"]["toy:lua-3"]["runtime_policy"],"Lua interpreted")
        self.assertEqual(result["programs"]["toy:lua-3"]["qualification"]["sample_count"],3)
        self.assertEqual(self.data,before)

    def test_changed_source_bytes_are_rejected(self):
        Path(self.audit["candidates"][2]["source_files"][0]["path"]).write_text("changed\n")
        with self.assertRaisesRegex(ValueError,"source bytes changed"):self.apply()

    def test_new_required_work_exclusion_is_not_readmitted(self):
        self.data["exclusions"]=[dict(key="toy:lua-3",reason="work mismatch")]
        with self.assertRaisesRegex(ValueError,"required-work review excludes"):self.apply()

    def test_changed_reviewed_snapshot_is_rejected(self):
        Path(self.audit["current_data"]["path"]).write_text("{}")
        with self.assertRaisesRegex(ValueError,"snapshot changed"):self.apply()

    def fresh_control(self):
        path=self.root/'exact-control.mec';path.write_text('f(x<f64>) => <f64> := x + 1\n')
        identity=sha(path);metric=identity+':mech'
        row=self.audit['candidates'][0]
        row['fresh_paired_mech_control']=dict(source=str(path),source_sha256=identity,loc=None,
            samples_seconds=[.5,.6,.7],median_seconds=.6)
        row['fresh_paired_mech_control_source_identities']=[metric]
        self.audit['fresh_control_source_reconciliation']=[dict(source_identities=[metric],canonical_parser_loc=1)]
        self.audit['sources'][metric]=dict(sha256=identity)
        self.audit['review']['source_decisions'][metric]=dict(source_sha256=identity,normalized_loc=1)
        return row,path,metric

    def test_fresh_control_requires_exact_hash_keyed_review(self):
        row,path,metric=self.fresh_control()
        del self.audit['review']['source_decisions'][metric]
        with self.assertRaisesRegex(ValueError,'Missing source-keyed review'):validate_reviewed_audit(self.audit)

    def test_fresh_control_raw_loc_and_timing_preserved_with_separate_canonical_count(self):
        row,path,metric=self.fresh_control();raw=copy.deepcopy(row['fresh_paired_mech_control'])
        result=self.apply();control=result['programs'][row['key']]['fresh_paired_mech_control']
        self.assertEqual(row['fresh_paired_mech_control'],raw)
        self.assertIsNone(control['loc'])
        self.assertEqual(control['canonical_loc'],1)
        self.assertEqual(control['samples_seconds'],[.5,.6,.7])
        self.assertEqual(control['median_seconds'],.6)
        self.assertEqual(control['canonical_source_identities'],[metric])
        self.assertIn(control['source_sha256'],result['files'])
        self.assertEqual(result['programs'][row['key']]['files'],[row['source_files'][0]['sha256']])

    def test_changed_fresh_control_source_is_rejected(self):
        row,path,metric=self.fresh_control();path.write_text('changed control\n')
        with self.assertRaisesRegex(ValueError,'Fresh paired control source bytes changed'):self.apply()

    def test_fresh_control_cannot_borrow_another_reviewed_source_count(self):
        row,path,metric=self.fresh_control()
        row['fresh_paired_mech_control_source_identities']=self.audit['candidates'][1]['source_identities']
        with self.assertRaisesRegex(ValueError,'bound to another source identity'):self.apply()


if __name__=="__main__":
    with timing_boundary() as lock:
        MUTEX_FD=lock.fileno()
        unittest.main()
