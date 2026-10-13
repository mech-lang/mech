import unittest
from pathlib import Path
from input_inventory import audit_command,processing_inputs,completed_owned_receipt,AUDIT_FLAGS


class InputInventoryFixtures(unittest.TestCase):
    def command(self):
        return ['/vendor/bin/nim','c','--mm:orc','--threads:on','--nimcache:/old/cache','--out:/old/bin','--lib:/vendor/lib','/authored/main.nim']

    def text(self):
        return 'Hint:  (toplevel): import: /authored/main.nim [Processing]\nHint: > main: import: /vendor/lib/system.nim [Processing]\nHint: > main: import: /authored/glue.nim [Processing]\nHint: >> glue: include: /vendor/wrapper/header.nim [Processing]\n'

    def parse(self,text=None,helpers=None):
        return processing_inputs(self.text() if text is None else text,'/authored/main.nim',
            ['/authored/glue.nim'] if helpers is None else helpers,['/vendor/lib','/vendor/wrapper'])

    def test_fresh_frontend_command_preserves_original_semantics(self):
        original = self.command(); command = audit_command(original,Path('/new/cache'),Path('/new/never-binary'))
        self.assertEqual(original,self.command())
        self.assertEqual(command[-6:-1],list(AUDIT_FLAGS)); self.assertEqual(command[-1],original[-1])
        self.assertEqual(command[2:4],original[2:4]); self.assertIn('--out:/new/never-binary',command)
        for bad in (original[:-1]+['--out:/another',original[-1]],original[:-1]+['--compileOnly:on',original[-1]],['/nim','r',*original[2:]]):
            with self.assertRaises(AssertionError): audit_command(bad,Path('/new/cache'),Path('/new/bin'))

    def test_import_include_and_repeated_includes_preserve_actual_owners(self):
        result = self.parse(self.text()+'Hint: >> glue: include: /vendor/wrapper/header.nim [Processing]\n')
        self.assertEqual(len(result['events']),5); self.assertEqual(len(result['files']),4)
        self.assertEqual([item['role'] for item in result['authored_files']],['application','application-helper'])
        included = [item for item in result['files'] if item['path'].endswith('header.nim')][0]
        self.assertEqual(included['event_indices'],[3,4]); self.assertFalse(included['counted'])

    def test_unknown_or_vendor_prefix_escape_is_rejected(self):
        for path in ('/foreign/kernel.nim','/vendor/library-secret.nim','/vendor/lib/../../foreign.nim'):
            with self.assertRaises(AssertionError): self.parse(self.text()+f'Hint: > main: import: {path} [Processing]\n')

    def test_main_and_all_authored_helpers_are_mandatory(self):
        for text in (self.text().replace('Hint: > main: import: /authored/glue.nim [Processing]\n',''),
                self.text().replace('Hint:  (toplevel): import: /authored/main.nim [Processing]\n','')):
            with self.assertRaises(AssertionError): self.parse(text)
        with self.assertRaises(AssertionError): self.parse(helpers=['/authored/glue.nim','/authored/hidden.nim'])

    def test_unrecognized_cached_relative_and_ansi_events_are_rejected(self):
        for changed in (self.text().replace(': import:',': import(dirty):',1),
                self.text().replace('/authored/main.nim','main.nim'),
                self.text().replace('Hint:','\x1b[31mHint:',1),
                self.text()+'Hint: cached: /vendor/lib/secret.nim [Processing]\n'):
            with self.assertRaises(AssertionError): self.parse(changed)

    def test_saved_owned_receipt_rejects_nonzero_boolean_and_cleanup(self):
        command = self.command()
        value = dict(command=command,inherited_common_mutex=True,preparation_only=True,never_chart_eligible=True,
            exit_status=0,final_exit_status=0)
        self.assertTrue(completed_owned_receipt(value,command))
        for key,wrong in (('exit_status',False),('final_exit_status',1),('inherited_common_mutex',False),('cleanup_errors',[{'error':'owned live child'}]),('command',command+['wrong'])):
            with self.assertRaises(AssertionError): completed_owned_receipt(dict(value,**{key:wrong}),command)
