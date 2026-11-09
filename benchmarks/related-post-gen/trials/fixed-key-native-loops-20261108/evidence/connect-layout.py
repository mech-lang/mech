from pathlib import Path
S=Path('/private/tmp/mech-related-fixed-key-native-loops-compiler-20261108/hosts/gpu/src/native_artifact')
def update(path,f):
 p=S/path;t=p.read_text();p.write_text(f(t))
def sub(path,a,b,n=1):
 def f(t):assert t.count(a)==n,(path,a[:80],t.count(a));return t.replace(a,b)
 update(path,f)
sub(Path('program/prepare/mod.rs'),'#[cfg(test)]\npub(in crate::native_artifact) mod fixed_key_loops;','pub(in crate::native_artifact) mod fixed_key_loops;')
sub(Path('program/prepare/mod.rs'),'    pub(in crate::native_artifact) borrowed_dictionary_values: borrowed_dictionary_values::Layout,','    pub(in crate::native_artifact) borrowed_dictionary_values: borrowed_dictionary_values::Layout,\n    pub(in crate::native_artifact) fixed_key_loops: fixed_key_loops::Layout,')
def prep(t):
 a=t.index('        let matrix_map_keys = matrix_map_keys::Layout::derive(');b=t.index('        let call_routes = call_routes::derive(',a);block=t[a:b];t=t[:a]+t[b:]
 block=block.replace('map_owner_words.as_ref()','Some(&map_owner_words)')
 old='''        let generic_dictionary = map_owner_words
            .as_ref()
            .map(|owners| generic_dictionary::Layout::derive(&plan, owners, &utf8_reads))
            .transpose()
            .map_err(str::to_owned)?
            .unwrap_or_default();'''
 new='''        let generic_dictionary = generic_dictionary::Layout::derive(&plan, &map_owner_words, &utf8_reads).map_err(str::to_owned)?;''';assert old in block;block=block.replace(old,new)
 old='''        let dictionary_seed_frames = map_owner_words
            .as_ref()
            .map(|owners| dictionary_seed_frames::Layout::derive(&plan, owners))
            .transpose()
            .map_err(str::to_owned)?
            .unwrap_or_default();''';new='''        let dictionary_seed_frames = dictionary_seed_frames::Layout::derive(&plan, &map_owner_words).map_err(str::to_owned)?;''';assert old in block;block=block.replace(old,new)
 block+='''        let fixed_key_loops = fixed_key_loops::Layout::derive(&plan,Some(&map_owner_words),&borrowed_dictionary_values,&matrix_map_keys).map_err(str::to_owned)?;
'''
 at=t.index('        let closed_tails = native::program::prepare::tail::Layout::derive(');t=t[:at]+block+t[at:]
 needle='''            &calls,
        )?;
        compose_work(''';assert t.count(needle)==1;t=t.replace(needle,'''            &calls,
            &fixed_key_loops,
        )?;
        compose_work(''')
 t=t.replace('            borrowed_dictionary_values,','            borrowed_dictionary_values,\n            fixed_key_loops,');return t
update(Path('program/prepare/mod.rs'),prep)
def wire(t):
 a=t.index('    let matrix_map_keys = if OBJECT_BOUND {');b=t.index('    if (image.version >= 7)',a);block=t[a:b];t=t[:a]+t[b:]
 block=block.replace('map_owner_words.as_ref()','raw_owners')
 a=block.index('    let generic_dictionary = map_owner_words');b=block.index('    let borrowed_dictionary_values =',a)
 block=block[:a]+'''    let generic_dictionary = raw_owners.map(|owners|native::program::prepare::generic_dictionary::Layout::derive(&plan, owners,&utf8_reads)).transpose().map_err(str::to_owned)?.unwrap_or_default();
    let dictionary_seed_frames = raw_owners.map(|owners|native::program::prepare::dictionary_seed_frames::Layout::derive(&plan,owners)).transpose().map_err(str::to_owned)?.unwrap_or_default();
'''+block[b:]
 block='''    let raw_owners=if frozen_layouts {w.map_owner_words.as_ref()}else{computed_map_owner_words.as_ref()};
'''+block+'''    let fixed_key_loops=native::program::prepare::fixed_key_loops::Layout::derive(&plan,raw_owners,&borrowed_dictionary_values,&matrix_map_keys).map_err(str::to_owned)?;
'''
 at=t.index('    if !OBJECT_BOUND {\n        native::program::prepare::tail::validate_image(');t=t[:at]+block+t[at:]
 needle='''            &closed_tails,
        )?;''';assert t.count(needle)==1;t=t.replace(needle,'''            &closed_tails,
            &fixed_key_loops,
        )?;''')
 t=t.replace('            borrowed_dictionary_values,','            borrowed_dictionary_values,\n            fixed_key_loops,');return t
update(Path('program/image/wire/mod.rs'),wire)
sub(Path('program/prepare/tail.rs'),'''        calls: &[Option<mech_engine::native_artifact::NativePreparedLibraryCall>],
    ) -> Result<Self, &'static str> {''','''        calls: &[Option<mech_engine::native_artifact::NativePreparedLibraryCall>],
        fixed_keys: &super::fixed_key_loops::Layout,
    ) -> Result<Self, &'static str> {''')
sub(Path('program/prepare/tail.rs'),'''                    spans.len() == 1
                        && owners.regions()[owner]''','''                    fixed_keys.owner(owner).is_some() || (spans.len() == 1
                        && owners.regions()[owner]''')
sub(Path('program/prepare/tail.rs'),'''                            .all(|&site| owners.call_span(site as usize) == Some(spans[0].0))
                });''','''                            .all(|&site| owners.call_span(site as usize) == Some(spans[0].0)))
                });''')
sub(Path('program/prepare/tail.rs'),'''                        owners.action_region(site as usize) == Some(owner)
''','''                        fixed_keys.owner(owner).is_some_and(|p|p.cell==site as usize)
                            || owners.action_region(site as usize) == Some(owner)
''')
sub(Path('program/prepare/tail.rs'),'''    serialized: &Layout,
) -> Result<(), String> {''','''    serialized: &Layout,
    fixed_keys: &super::fixed_key_loops::Layout,
) -> Result<(), String> {''')
sub(Path('program/prepare/tail.rs'),'Layout::derive(plan, &borrowed, &visits, &aggregates, owners, frames, calls)?','Layout::derive(plan, &borrowed, &visits, &aggregates, owners, frames, calls, fixed_keys)?')
p=S/'program/prepare/frozen_map/tests.rs';t=p.read_text();at=t.index('crate::native_artifact::program::prepare::tail::Layout::derive(');end=t.index('    )',at);t=t[:end]+t[end:].replace('    )','        &Default::default(),\n    )',1);p.write_text(t)
# This temporary refusal keeps a newly described region on its original path
# until its current-funded native owner is connected in the next edit.
sub(Path('program/runtime/maps/owner/closed.rs'),'''        if !core::ptr::eq(self.descriptor, ctx.descriptor())''','''        if self.descriptor.metadata().fixed_key_loops.owner(index).is_some(){ return Ok(None); }
        if !core::ptr::eq(self.descriptor, ctx.descriptor())''')
