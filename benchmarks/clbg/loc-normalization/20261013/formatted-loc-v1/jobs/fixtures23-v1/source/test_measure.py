"""Four source fixtures for the new formatter adapter; never run unlocked."""
from pathlib import Path
from types import SimpleNamespace
import unittest
import measure


class FormatterBoundary(unittest.TestCase):
    def test_import_origins_reject_unpinned_external_and_installed_packages(self):
        root = Path('/qualified-python/lib/python3.12')
        names = {'statistics', 'unittest', 'math'}
        self.assertTrue(measure.standard_origin('unittest.case', root / 'unittest/case.py', root, names))
        self.assertTrue(measure.standard_origin('math', root / 'lib-dynload/math.so', root, names))
        for name, path in (('statistics', Path('/external/statistics.py')),
                           ('statistics', root / 'site-packages/statistics.py'),
                           ('math', root / 'dist-packages/math.so'),
                           ('foreign_lexer', root / 'foreign_lexer.py'),
                           ('foreign_lexer', Path('/external/foreign_lexer.py'))):
            with self.subTest(name=name, path=str(path)):
                self.assertFalse(measure.standard_origin(name, path, root, names))

    def test_only_isolated_source_placeholders_and_literal_argv(self):
        recipe=dict(output_mode='file',arguments=['--style=LLVM','{input}','--out={output}'])
        self.assertEqual(measure.command_for(recipe,'/tool',Path('/copy/a.c'),Path('/copy/formatted.c')),
                         ['/tool','--style=LLVM','/copy/a.c','--out=/copy/formatted.c'])
        for changed in (dict(recipe,arguments=['{source}']),dict(recipe,arguments=['--all']),
                        dict(recipe,output_mode='benchmark')):
            with self.assertRaises(ValueError):
                measure.command_for(changed,'/tool',Path('/copy/a.c'),Path('/copy/formatted.c'))

    def test_physical_lines_do_not_call_semantic_normalization(self):
        calls=[]
        def mask(text,language):
            calls.append((text,language))
            return dict(cleaned='     \nx=1; y=2;\n\n"SSS"\n',diagnostics=[])
        def forbidden(*args):
            raise AssertionError('A semantic normalizer must never run')
        loaded={'lexical':SimpleNamespace(mask_source=mask,normalize=forbidden,__file__='/qualified/lexical.py')}
        counter=dict(module='lexical',module_path='/qualified/lexical.py',function='mask_source',argument='c')
        source='//hi \nx=1; y=2;\n\n"abc"\n'
        self.assertEqual(measure.physical(source,counter,loaded),2)
        self.assertEqual(calls,[(source,'c')])
        with self.assertRaises(ValueError):
            measure.physical(source,dict(counter,function='normalize'),loaded)
        with self.assertRaises(ValueError):
            measure.physical(source,dict(counter,module_path='/other/lexical.py'),loaded)
        # A successfully formatted empty helper is valid UTF-8 source with
        # zero physical lines; the complete program must still total positive.
        loaded['lexical'].mask_source=lambda text,lang:dict(cleaned=text,diagnostics=[])
        self.assertEqual(measure.physical('',counter,loaded),0)
        loaded['lexical'].mask_source=lambda text,lang:dict(cleaned=text,diagnostics=[{'kind':'unknown-token'}])
        with self.assertRaises(ValueError):
            measure.physical(source,counter,loaded)
        # Equal character length alone must not let a lexical mask manufacture
        # or remove a physical source row.
        loaded['lexical'].mask_source=lambda text,lang:dict(cleaned=text.replace('\n',' '),diagnostics=[])
        with self.assertRaises(ValueError):
            measure.physical(source,counter,loaded)

    def test_process_success_requires_literal_mutex_and_complete_cleanup(self):
        command=['/formatter','/copy/a.c']
        detail=dict(command=command,inherited_common_mutex=True,exit_status=0,final_exit_status=0)
        measure.successful(detail,command)
        for change in ({'inherited_common_mutex':1},{'exit_status':False},{'final_exit_status':None},
                       {'cleanup_error':'owned reap failed'},{'remaining_group_members':[123]},
                       {'command':['/formatter','/scored/original.c']}):
            with self.assertRaises(ValueError):
                measure.successful(dict(detail,**change),command)
