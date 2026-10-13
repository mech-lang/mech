# Source-only proposal extractor, not executed and not a qualifier.
# $data: reviewed immutable original dataset JSON (slurpfile).
# $reports: caller-supplied exact relative report -> preserved JSON mapping.
# Runtime/build cross-links remain unqualified until the guarded route review.
def chosen($language): ["c","cpp","rust","mech","mech-jit","mech-resident","julia","javascript"]|index($language);
def rows($report):
  if ($report.programs|type)=="array" then $report.programs[]
  elif ($report.programs|type)=="object" then $report.programs|to_entries[].value
  elif ($report.results|type)=="array" then $report.results[]
  else empty end;
{
  status:"source-only-existing-platform-evidence-proposals",
  verified:false, canonical_LOC_admitted:false, chart_admission:false,
  records:[
    $data[0].programs|to_entries[]|. as $entry|$entry.value as $program|
    select(chosen($program.language)!=null)|
    $reports[$program.report] as $report|
    [rows($report)|select((.id==$program.id or .key==$program.key or
       ($program.language=="mech-resident" and .source_key==($program.id|rtrimstr(":resident")) and .phase=="after")) and
       (.benchmark==$program.benchmark or ($report.benchmark==$program.benchmark)) and
       .samples_seconds==$program.samples)] as $matches|
    {key:$entry.key,language:$program.language,benchmark:$program.benchmark,
     report:$program.report,expected_report_sha256:$data[0].provenance[$program.report],
     source_report:$program.source_report,matched_saved_rows:($matches|length),verified:false,
     displayed_source_files:[$program.files[] as $file|$data[0].files[$file]|{path,sha256}],
     displayed_samples:$program.samples,displayed_seconds:$program.seconds,
     actual_saved_records:[$matches[]|{id,key,source_key,phase,mode,command,compile_command,
       source,source_sha256,binary,binary_sha256,executable_sha256,command_assets,
       runtime_environment,environment_overrides,timing_boundary,samples_seconds}],
     historical_asset_pins:([ $matches[].command_assets[]? ]|unique|map(. as $path|{path:$path,sha256:$report.asset_hashes[$path]})),
     historical_toolchains:($report.toolchains//{}),
     proposed_confidence:{historical_platform_verified:false,historical_execution_verified:false,historical_executable_bytes_verified:false},
     unknown_reasons:([if ($matches|length)!=1 then "ambiguous-report-record-or-origin" else empty end,
       if $program.language=="javascript" then "unrecorded-legacy-runtime-execution-options" else empty end])}
  ],
  proof_rule:"All exact current identities yield completed or unknown proposals. Saved original report/source/build/command/runtime/sample pins must be independently qualified; path, version, language name or present artifact alone cannot prove missing historical byte identity. No source/sample/timing mutation."
}
