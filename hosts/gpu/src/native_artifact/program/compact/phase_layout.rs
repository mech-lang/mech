//! Optional layout profiles for diagnostic AArch64 phases.

use super::profile_inputs::SelectedProfiles;
use crate::native_artifact::codegen::compilation_inputs::CompilationInputs;
use cranelift_codegen::Context;
use std::{io, path::Path};

#[derive(serde::Deserialize)]
struct Profile {
    order: Vec<u32>,
    literal_offset: Option<u32>,
    #[serde(default)]
    thread_jumps: bool,
    object_tail: Option<ObjectTail>,
}

#[derive(serde::Deserialize)]
struct ObjectTail {
    code_bytes: usize,
    reference_code_bytes: usize,
    alignment: u8,
    #[serde(default)]
    nop_fill: bool,
    #[serde(default)]
    exact_root_extent: bool,
}

pub(super) fn profiles(compiler_inputs: &CompilationInputs) -> Option<String> {
    compiler_inputs.var("MECH_COMPACT_PHASE_BLOCK_LAYOUT").ok()
}

pub(super) fn apply_fp_spill_cache(
    compiler_inputs: &CompilationInputs,
    context: &mut Context,
    symbol: &str,
) -> io::Result<()> {
    context.func.aarch64_fp_spill_keep_pairs = compiler_inputs
        .var_os("MECH_COMPACT_FP_SPILL_KEEP_PAIRS")
        .is_some();
    if let Ok(selection) = compiler_inputs.var("MECH_COMPACT_FP_SPILL_SLOTS") {
        for entry in selection.split(';') {
            let (name, count) = entry
                .split_once('@')
                .ok_or_else(|| io::Error::other("expected phase@spill-cell-count"))?;
            let count = count.parse::<u8>().map_err(io::Error::other)?;
            if name == "all" || name == symbol {
                context.func.aarch64_fp_spill_slots = count;
            }
        }
    }
    Ok(())
}

fn read(
    selected_profiles: &SelectedProfiles,
    profiles: &str,
    symbol: &str,
) -> io::Result<Option<Profile>> {
    profiles
        .split(';')
        .filter_map(|entry| entry.split_once('@'))
        .find_map(|(name, path)| (name == symbol).then_some(path))
        .map(|path| {
            Ok(serde_json::from_slice(
                selected_profiles
                    .resolve_selected(Path::new(path))
                    .bytes()?,
            )?)
        })
        .transpose()
}

pub(super) fn apply(
    compiler_inputs: &CompilationInputs,
    selected_profiles: &SelectedProfiles,
    context: &mut Context,
    symbol: &str,
) -> io::Result<()> {
    if let Some(profiles) = profiles(compiler_inputs)
        && let Some(profile) = read(selected_profiles, &profiles, symbol)?
    {
        context.func.aarch64_block_layout = profile.order;
        context.func.aarch64_block_layout_literal_start = profile.literal_offset;
        context.func.aarch64_block_layout_thread_jumps = profile.thread_jumps;
    }
    Ok(())
}

/// Register weights refer to the full lowered VCode, rather than byte offsets
/// in a previously allocated machine body. Validation occurs after lowering.
pub(super) fn apply_register_weights(
    compiler_inputs: &CompilationInputs,
    selected_profiles: &SelectedProfiles,
    context: &mut Context,
    symbol: &str,
) -> io::Result<()> {
    #[derive(serde::Deserialize)]
    struct RegisterWeights {
        expected_vcode: String,
        block_frequencies: Vec<u32>,
    }
    if let Ok(profiles) = compiler_inputs.var("MECH_COMPACT_PHASE_REGISTER_WEIGHTS")
        && let Some(path) = profiles
            .split(';')
            .filter_map(|entry| entry.split_once('@'))
            .find_map(|(name, path)| (name == symbol).then_some(path))
    {
        let weights: RegisterWeights = serde_json::from_slice(
            selected_profiles
                .resolve_selected(Path::new(path))
                .bytes()?,
        )?;
        context.func.regalloc_block_frequencies =
            Some((weights.expected_vcode, weights.block_frequencies));
    }
    Ok(())
}

pub(super) fn pad_object_tail(
    selected_profiles: &SelectedProfiles,
    profiles: &str,
    symbol: &str,
    code: &mut Box<[u8]>,
) -> io::Result<()> {
    if let Some(profile) = read(selected_profiles, profiles, symbol)?
        && let Some(tail) = profile.object_tail
    {
        if profile.order.is_empty() {
            return Err(io::Error::other(
                "object tail padding requires a block layout",
            ));
        }
        let padding = tail.padding(code.len()).ok_or_else(|| {
            io::Error::other(format!(
                "object tail padding profile does not match {symbol}"
            ))
        })?;
        // The block layout verifies the leaf's transfers and literal pool.
        // Pad beyond that buffer, leaving code offsets and accounting intact.
        let extent = code.len();
        append_padding(code, extent, padding);
    }
    Ok(())
}

pub(super) fn pad_checked_root_tail(
    selected_profiles: &SelectedProfiles,
    profile: &Path,
    alignment: u32,
    code: &mut Box<[u8]>,
) -> io::Result<()> {
    let tail: ObjectTail =
        serde_json::from_slice(selected_profiles.resolve_selected(profile).bytes()?)?;
    let alignment = alignment as usize;
    let padding = tail
        .padding(code.len())
        .filter(|_| {
            alignment >= 4
                && alignment.is_power_of_two()
                && alignment <= usize::from(tail.alignment)
                && (!tail.exact_root_extent
                    || tail.reference_code_bytes - code.len() < usize::from(tail.alignment))
        })
        .ok_or_else(|| io::Error::other("object tail profile does not match the checked root"))?;
    // The measured tail normally follows the rounded function extent.
    // An exact profile restores the reference extent directly.
    let bytes = code.len();
    let extent = if tail.exact_root_extent {
        bytes
    } else {
        bytes.next_multiple_of(alignment)
    };
    if tail.nop_fill {
        append_padding(code, bytes, padding + (extent - bytes));
    } else {
        append_padding(code, extent, padding);
    }
    Ok(())
}

fn append_padding(code: &mut Box<[u8]>, extent: usize, padding: usize) {
    const NOP: [u8; 4] = 0xd503_201f_u32.to_le_bytes();
    if padding != 0 {
        let mut bytes = std::mem::take(code).into_vec();
        bytes.resize(extent, 0);
        for _ in 0..padding / NOP.len() {
            bytes.extend_from_slice(&NOP);
        }
        *code = bytes.into_boxed_slice();
    }
}

impl ObjectTail {
    fn padding(&self, code_bytes: usize) -> Option<usize> {
        let alignment = usize::from(self.alignment);
        (self.code_bytes == code_bytes
            && code_bytes % 4 == 0
            && self.reference_code_bytes >= code_bytes
            && self.reference_code_bytes % 4 == 0
            && alignment >= 4
            && alignment.is_power_of_two())
        .then(|| (self.reference_code_bytes - code_bytes) % alignment)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, fs, path::PathBuf, rc::Rc};

    #[test]
    fn restores_reference_extent_residue_and_rejects_stale_size() {
        let tail = ObjectTail {
            code_bytes: 496,
            reference_code_bytes: 592,
            alignment: 64,
            nop_fill: false,
            exact_root_extent: false,
        };
        assert_eq!(tail.padding(496), Some(32));
        assert_eq!(tail.padding(512), None);
        assert_eq!(
            ObjectTail {
                alignment: 24,
                ..tail
            }
            .padding(496),
            None
        );
    }

    const LAYOUT: &[u8] = br#"{"order":[0,1,2],"literal_offset":32,"thread_jumps":false}"#;

    fn layout_inputs(value: String) -> CompilationInputs {
        CompilationInputs::capture([("MECH_COMPACT_PHASE_BLOCK_LAYOUT".into(), value.into())])
    }

    #[test]
    fn only_the_first_selected_layout_is_read_and_survives_mutation() {
        for delete in [false, true] {
            let folder = tempfile::tempdir().unwrap();
            let first = folder.path().join("first.json");
            let ignored = folder.path().join("unused.json");
            let raw = layout_inputs(format!(
                "malformed;unused@{ignored};selected@{first};selected@{ignored}",
                first = first.display(),
                ignored = ignored.display(),
            ));
            let cloned = raw.clone();
            fs::write(&first, LAYOUT).unwrap();
            let reads = Rc::new(RefCell::new(Vec::new()));
            let ledger = reads.clone();
            let selected = SelectedProfiles::with_reader(move |path| {
                ledger.borrow_mut().push(path.to_owned());
                let contents = fs::read(path)?;
                if delete {
                    fs::remove_file(path)?;
                } else {
                    fs::write(path, b"invalid")?;
                }
                Ok(contents)
            });
            for setting in [&raw, &cloned] {
                let mut context = Context::new();
                apply(setting, &selected, &mut context, "irrelevant").unwrap();
                apply(setting, &selected, &mut context, "selected").unwrap();
                assert_eq!(context.func.aarch64_block_layout, [0, 1, 2]);
                assert_eq!(context.func.aarch64_block_layout_literal_start, Some(32));
                assert!(!context.func.aarch64_block_layout_thread_jumps);
            }
            assert_eq!(*reads.borrow(), [first]);
        }
    }

    #[test]
    fn selected_error_is_cached_until_a_new_compilation() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("missing-first.json");
        let raw = layout_inputs(format!("selected@{}", path.display()));
        let ledger = Rc::new(RefCell::new(Vec::<PathBuf>::new()));
        let reader_ledger = ledger.clone();
        let selected = SelectedProfiles::with_reader(move |path| {
            reader_ledger.borrow_mut().push(path.to_owned());
            let result = fs::read(path);
            fs::write(path, LAYOUT).unwrap();
            result
        });
        let mut context = Context::new();
        apply(&raw, &selected, &mut context, "unmatched").unwrap();
        assert!(ledger.borrow().is_empty());
        let first = apply(&raw, &selected, &mut context, "selected").unwrap_err();
        let second = apply(&raw, &selected, &mut context, "selected").unwrap_err();
        assert_eq!(first.kind(), io::ErrorKind::NotFound);
        assert_eq!(first.kind(), second.kind());
        assert_eq!(first.raw_os_error(), second.raw_os_error());
        assert_eq!(first.to_string(), second.to_string());
        assert_eq!(*ledger.borrow(), [path]);
        apply(&raw, &SelectedProfiles::new(), &mut context, "selected").unwrap();
        assert_eq!(context.func.aarch64_block_layout, [0, 1, 2]);
    }

    #[test]
    fn reached_consumers_preserve_parse_and_tail_validation_order() {
        let selected = SelectedProfiles::with_reader(|path| {
            match path.to_str().unwrap() {
            "malformed" => Ok(b"invalid".to_vec()),
            "layout" => Ok(br#"{"order":[],"object_tail":{"code_bytes":4,"reference_code_bytes":8,"alignment":16}}"#.to_vec()),
            "root" => Ok(br#"{"code_bytes":4,"reference_code_bytes":8,"alignment":16}"#.to_vec()),
            _ => unreachable!(),
        }
        });
        let raw = layout_inputs("other@malformed;selected@layout".into());
        let mut context = Context::new();
        apply(&raw, &selected, &mut context, "selected").unwrap();
        assert!(context.func.aarch64_block_layout.is_empty());
        let mut code = vec![0; 8].into_boxed_slice();
        let error = pad_object_tail(&selected, &profiles(&raw).unwrap(), "selected", &mut code)
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "object tail padding requires a block layout"
        );
        assert!(apply(&raw, &selected, &mut context, "other").is_err());
        let error = pad_checked_root_tail(&selected, Path::new("root"), 4, &mut code).unwrap_err();
        assert_eq!(
            error.to_string(),
            "object tail profile does not match the checked root"
        );
        assert_eq!(&*code, &[0; 8]);
    }

    #[test]
    fn explicit_profile_inputs_ignore_a_fixed_opposing_environment() {
        use std::{env, process::Command};

        const MODE: &str = "NATIVE_PROFILE_ISOLATION_CHILD";
        const OUTPUT: &str = "NATIVE_PROFILE_ISOLATION_OUTPUT";
        const LAYOUT_KEY: &str = "MECH_COMPACT_PHASE_BLOCK_LAYOUT";
        const WEIGHTS_KEY: &str = "MECH_COMPACT_PHASE_REGISTER_WEIGHTS";
        fn settings(folder: &Path, mode: &str) -> [(std::ffi::OsString, std::ffi::OsString); 2] {
            let layout = format!(
                "selected@{};missing@{}",
                folder.join(format!("layout-{mode}")).display(),
                folder.join("missing-profile").display()
            );
            let weights = format!(
                "selected@{}",
                folder.join(format!("weights-{mode}")).display()
            );
            [
                (LAYOUT_KEY.into(), layout.into()),
                (WEIGHTS_KEY.into(), weights.into()),
            ]
        }
        fn applied(raw: &CompilationInputs) -> serde_json::Value {
            let paths = Rc::new(RefCell::new(Vec::new()));
            let ledger = paths.clone();
            let selected = SelectedProfiles::with_reader(move |path| {
                ledger.borrow_mut().push(path.to_owned());
                fs::read(path)
            });
            let mut context = Context::new();
            apply(raw, &selected, &mut context, "unselected").unwrap();
            apply_register_weights(raw, &selected, &mut context, "unselected").unwrap();
            assert!(paths.borrow().is_empty());
            apply(raw, &selected, &mut context, "selected").unwrap();
            apply_register_weights(raw, &selected, &mut context, "selected").unwrap();
            let selected_paths = paths.borrow().clone();
            let error = apply(raw, &selected, &mut context, "missing").unwrap_err();
            let (weight_id, weights) = context.func.regalloc_block_frequencies.as_ref().unwrap();
            serde_json::json!({
                "order": context.func.aarch64_block_layout,
                "literal_offset": context.func.aarch64_block_layout_literal_start,
                "thread_jumps": context.func.aarch64_block_layout_thread_jumps,
                "expected_vcode": weight_id, "block_frequencies": weights,
                "selected_paths": selected_paths,
                "missing": {"kind": format!("{:?}", error.kind()), "os_code": error.raw_os_error(),
                            "message": error.to_string(), "read_path": paths.borrow().last()},
            })
        }
        if let Some(mode) = env::var_os(MODE) {
            let path = PathBuf::from(env::var_os(OUTPUT).unwrap());
            let folder = path.parent().unwrap();
            let rows = match mode.to_str().unwrap() {
                "legacy-a" | "legacy-b" => vec![applied(&CompilationInputs::legacy())],
                "explicit-ambient-a" | "explicit-ambient-b" => {
                    let a = CompilationInputs::capture(settings(folder, "a"));
                    let b = CompilationInputs::capture(settings(folder, "b"));
                    [&a, &b, &a].into_iter().map(applied).collect()
                }
                _ => panic!("unexpected child mode {mode:?}"),
            };
            fs::write(path, serde_json::to_vec(&rows).unwrap()).unwrap();
            return;
        }

        let folder = tempfile::tempdir().unwrap();
        let base = folder.path();
        fs::write(base.join("layout-a"), LAYOUT).unwrap();
        fs::write(
            base.join("layout-b"),
            br#"{"order":[0,2,1],"literal_offset":64,"thread_jumps":true}"#,
        )
        .unwrap();
        fs::write(
            base.join("weights-a"),
            br#"{"expected_vcode":"profile-a","block_frequencies":[100,10,1]}"#,
        )
        .unwrap();
        fs::write(
            base.join("weights-b"),
            br#"{"expected_vcode":"profile-b","block_frequencies":[200,1,20]}"#,
        )
        .unwrap();
        let module = module_path!().split_once("::").unwrap().1;
        let test = format!("{module}::explicit_profile_inputs_ignore_a_fixed_opposing_environment");
        let child = |mode: &str, ambient: &str| -> Vec<serde_json::Value> {
            let path = base.join(mode);
            let mut command = Command::new(env::current_exe().unwrap());
            command
                .args(["--exact", &test, "--test-threads=1"])
                .env(MODE, mode)
                .env(OUTPUT, &path);
            for (key, _) in env::vars_os() {
                if key
                    .to_ascii_uppercase()
                    .as_encoded_bytes()
                    .starts_with(b"MECH_")
                {
                    command.env_remove(key);
                }
            }
            let result = command.envs(settings(base, ambient)).output().unwrap();
            assert!(
                result.status.success(),
                "{mode}: {}{}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
            serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
        };
        let [a]: [_; 1] = child("legacy-a", "a").try_into().unwrap();
        let [b]: [_; 1] = child("legacy-b", "b").try_into().unwrap();
        for (row, mode, order, offset, thread, weights) in [
            (&a, "a", [0, 1, 2], 32, false, [100, 10, 1]),
            (&b, "b", [0, 2, 1], 64, true, [200, 1, 20]),
        ] {
            assert_eq!(row["order"], serde_json::json!(order));
            assert_eq!(row["literal_offset"], offset);
            assert_eq!(row["thread_jumps"], thread);
            assert_eq!(row["expected_vcode"], format!("profile-{mode}"));
            assert_eq!(row["block_frequencies"], serde_json::json!(weights));
            assert_eq!(
                row["selected_paths"],
                serde_json::json!([
                    base.join(format!("layout-{mode}")),
                    base.join(format!("weights-{mode}"))
                ])
            );
            assert_eq!(row["missing"]["kind"], "NotFound");
            assert!(row["missing"]["os_code"].is_i64());
            assert!(!row["missing"]["message"].as_str().unwrap().is_empty());
            assert_eq!(
                row["missing"]["read_path"],
                serde_json::json!(base.join("missing-profile"))
            );
        }
        let expected = [a.clone(), b, a];
        assert_eq!(child("explicit-ambient-a", "a"), expected);
        assert_eq!(child("explicit-ambient-b", "b"), expected);
    }
}
