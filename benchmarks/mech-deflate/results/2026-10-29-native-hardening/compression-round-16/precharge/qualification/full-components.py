from pathlib import Path
import json,subprocess,time,hashlib
root=Path(__file__).resolve().parent
binary=root/'components/component-v2-tests'
command=[str(binary)]+['packed_update_qualification', 'buffer_frame_qualification', 'sequence_capacity_qualification', 'collection_qualification', 'uniform_words_qualification', 'word_read_qualification', 'narrow_word_qualification', 'matchfinder_qualification', 'assembled_compressor_source_preserves_its_pinned_bytes', 'scalar_state_qualification', 'tail_qualification', 'costs_qualification', 'default_costs_qualification', 'pinned_min_cost_paths_keep_exact_nodes_and_real_reservation', 'path_checks_preserve_entry_order_publication_aliases_and_recovery', 'argument_transport_qualification', 'shared_transport_qualification', 'numeric_write_qualification', 'numeric_tail_qualification', 'numeric_result_qualification', 'numeric_predicate_qualification', 'dynamic_projection_qualification', '--nocapture', '--test-threads=1']
start=time.monotonic();result=subprocess.run(command,capture_output=True,text=True,timeout=300)
(root/'full-components.log').write_text(result.stdout+result.stderr)
(root/'full-components.json').write_text(json.dumps({'command':command,'exit':result.returncode,'seconds':time.monotonic()-start,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'scope':'Same maintained round13 component cohort and unchanged allowances, plus current-predicate version selection and dynamic aggregate projection / current-selector / publication-retry qualification. Linked generated-project test is not in this cohort.'},indent=2)+'\n')
print(result.stdout[-4000:]+result.stderr[-3000:]);result.check_returncode()
