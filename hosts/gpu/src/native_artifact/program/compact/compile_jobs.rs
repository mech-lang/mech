//! Function-local machine compilation, with coordinator-owned Object publication.
use super::object::DirectRuntimeCalls;
use crate::native_artifact::{NativeCompileError, compiler::backend};
use cranelift_codegen::{
    Context,
    control::ControlPlane,
    ir::Signature,
    isa::{OwnedTargetIsa, TargetIsa},
};
use cranelift_module::{
    DataDescription, DataId, FuncId, Linkage, Module, ModuleDeclarations, ModuleReloc, ModuleResult,
};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::JoinHandle;

type Job = Box<dyn FnOnce(&dyn TargetIsa) + Send>;

pub(super) struct Pool {
    isa: OwnedTargetIsa,
    queue: Option<mpsc::SyncSender<Job>>,
    workers: Vec<JoinHandle<()>>,
    submitted: std::cell::Cell<usize>,
    cleanup_submitted: std::cell::Cell<usize>,
    direct_runtime_calls: DirectRuntimeCalls,
}

pub(super) struct Ticket<T = Context>(mpsc::Receiver<Result<T, NativeCompileError>>);

impl Pool {
    pub(super) fn new(
        isa: OwnedTargetIsa,
        count: usize,
        direct_runtime_calls: DirectRuntimeCalls,
    ) -> Result<Self, NativeCompileError> {
        assert!(count > 0);
        let (queue, jobs) = mpsc::sync_channel::<Job>(count);
        let jobs = Arc::new(Mutex::new(jobs));
        let mut pool = Self {
            isa,
            queue: Some(queue),
            workers: Vec::new(),
            submitted: std::cell::Cell::new(0),
            cleanup_submitted: std::cell::Cell::new(0),
            direct_runtime_calls,
        };
        for index in 0..count {
            let jobs = jobs.clone();
            let isa = pool.isa.clone();
            let worker = std::thread::Builder::new()
                .name(format!("mech-compile-{index}"))
                .spawn(move || {
                    loop {
                        let next = { jobs.lock().unwrap().recv() };
                        let Ok(job) = next else { break };
                        job(isa.as_ref());
                    }
                })
                .map_err(backend)?;
            pool.workers.push(worker);
        }
        Ok(pool)
    }

    pub(super) fn submit(&self, context: Context) -> Result<Ticket, NativeCompileError> {
        self.enqueue(move |isa| compile(context, isa))
    }

    pub(super) fn submit_work<T: Send + 'static>(
        &self,
        work: impl FnOnce(&dyn TargetIsa) -> Result<T, NativeCompileError> + Send + 'static,
    ) -> Result<Ticket<T>, NativeCompileError> {
        let ticket = self.enqueue(work)?;
        self.cleanup_submitted.set(self.cleanup_submitted.get() + 1);
        Ok(ticket)
    }

    fn enqueue<T: Send + 'static>(
        &self,
        work: impl FnOnce(&dyn TargetIsa) -> Result<T, NativeCompileError> + Send + 'static,
    ) -> Result<Ticket<T>, NativeCompileError> {
        let (reply, result) = mpsc::sync_channel(1);
        self.queue
            .as_ref()
            .unwrap()
            .send(Box::new(move |isa| {
                let _ = reply.send(work(isa));
            }))
            .map_err(backend)?;
        self.submitted.set(self.submitted.get() + 1);
        Ok(Ticket(result))
    }

    pub(super) fn record<M: Module>(
        &self,
        module: &M,
    ) -> Result<RecordedModule, NativeCompileError> {
        RecordedModule::new(
            module.declarations(),
            self.isa.clone(),
            self.direct_runtime_calls,
        )
        .map_err(backend)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_codegen::{
        ir::{AbiParam, ExtFuncData, ExternalName, InstBuilder, LibCall, types},
        settings::{self, Configurable},
    };
    use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
    use cranelift_module::default_libcall_names;
    use cranelift_object::{ObjectBuilder, ObjectModule};

    fn module() -> (ObjectModule, OwnedTargetIsa) {
        let mut flags = settings::builder();
        for (key, value) in [("opt_level", "speed"), ("is_pic", "true")] {
            flags.set(key, value).unwrap();
        }
        let isa = cranelift_native::builder()
            .unwrap()
            .finish(settings::Flags::new(flags))
            .unwrap();
        let module = ObjectModule::new(
            ObjectBuilder::new(
                isa.clone(),
                "ordered_compiler_jobs",
                default_libcall_names(),
            )
            .unwrap(),
        );
        (module, isa)
    }

    fn signature<M: Module>(module: &M) -> Signature {
        let mut signature = module.make_signature();
        signature.params.push(AbiParam::new(types::I64));
        signature.returns.push(AbiParam::new(types::I64));
        signature
    }

    fn body<M: Module>(module: &mut M, libcall: bool) -> Context {
        let mut context = module.make_context();
        context.func.signature = signature(module);
        let mut scratch = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut context.func, &mut scratch);
        let entry = builder.create_block();
        builder.append_block_params_for_function_params(entry);
        builder.switch_to_block(entry);
        let input = builder.block_params(entry)[0];
        if libcall {
            let mut signature = module.make_signature();
            signature.params = vec![
                AbiParam::new(types::I64),
                AbiParam::new(types::I32),
                AbiParam::new(types::I64),
            ];
            signature.returns.push(AbiParam::new(types::I64));
            let signature = builder.func.import_signature(signature);
            let function = builder.func.import_function(ExtFuncData {
                name: ExternalName::LibCall(LibCall::Memset),
                signature,
                colocated: false,
                patchable: false,
            });
            let zero = builder.ins().iconst(types::I32, 0);
            let length = builder.ins().iconst(types::I64, 8);
            builder.ins().call(function, &[input, zero, length]);
        }
        let result = builder.ins().iadd_imm(input, 1);
        builder.ins().return_(&[result]);
        builder.seal_all_blocks();
        builder.finalize();
        context
    }

    fn later<M: Module>(module: &mut M) -> (FuncId, Context) {
        let signature = signature(module);
        let second = module
            .declare_function("second", Linkage::Local, &signature)
            .unwrap();
        // Repeat/merge declarations, anonymous IDs and interleaved data must
        // replay exactly, including a data relocation to a later function.
        assert_eq!(
            module
                .declare_function("second", Linkage::Local, &signature)
                .unwrap(),
            second
        );
        let table = module
            .declare_data("table", Linkage::Local, false, false)
            .unwrap();
        let anonymous = module.declare_anonymous_data(false, false).unwrap();
        let nested = module.declare_anonymous_function(&signature).unwrap();
        let mut data = DataDescription::new();
        data.define(vec![0; 8].into_boxed_slice());
        let function = module.declare_func_in_data(second, &mut data);
        data.write_function_addr(0, function);
        module.define_data(table, &data).unwrap();
        let mut zero = DataDescription::new();
        zero.define_zeroinit(16);
        module.define_data(anonymous, &zero).unwrap();
        let mut nested_body = body(module, false);
        module.define_function(nested, &mut nested_body).unwrap();
        let context = body(module, false);
        (second, context)
    }

    fn image(count: usize, reverse_submission: bool) -> Vec<u8> {
        let (mut module, isa) = module();
        let signature = signature(&module);
        let first = module
            .declare_function("first", Linkage::Export, &signature)
            .unwrap();
        let mut first_body = body(&mut module, true);
        if count == 0 {
            module.define_function(first, &mut first_body).unwrap();
            let (second, mut context) = later(&mut module);
            module.define_function(second, &mut context).unwrap();
        } else {
            let pool = Pool::new(isa, count, DirectRuntimeCalls::default()).unwrap();
            let mut recorded = pool.record(&module).unwrap();
            let (second, context) = later(&mut recorded);
            let (first_job, second_job) = if reverse_submission {
                let second_job = pool.submit(context).unwrap();
                (pool.submit(first_body).unwrap(), second_job)
            } else {
                let first_job = pool.submit(first_body).unwrap();
                (first_job, pool.submit(context).unwrap())
            };
            publish(&mut module, first, &first_job.wait().unwrap()).unwrap();
            recorded.replay(&mut module).unwrap();
            publish(&mut module, second, &second_job.wait().unwrap()).unwrap();
        }
        module.finish().emit().unwrap()
    }

    #[test]
    fn workers_replay_every_object_event_in_original_order() {
        let baseline = image(0, false);
        for count in [1, 2, 4] {
            for reverse in [false, true] {
                for _ in 0..4 {
                    assert_eq!(image(count, reverse), baseline);
                }
            }
        }
    }

    #[test]
    fn earlier_publication_error_precedes_invalid_later_data() {
        let (mut module, isa) = module();
        let signature = signature(&module);
        let first = module
            .declare_function("imported", Linkage::Import, &signature)
            .unwrap();
        let mut context = body(&mut module, false);
        let expected = module
            .define_function(first, &mut context)
            .unwrap_err()
            .to_string();
        let pool = Pool::new(isa, 2, DirectRuntimeCalls::default()).unwrap();
        let first_job = pool.submit(body(&mut module, false)).unwrap();
        let mut recorded = pool.record(&module).unwrap();
        let data = recorded.declare_anonymous_data(false, false).unwrap();
        // Publishing this uninitialized description would panic. The earlier
        // original module error must return before later events are replayed.
        recorded.define_data(data, &DataDescription::new()).unwrap();
        let context = first_job.wait().unwrap();
        assert_eq!(
            publish(&mut module, first, &context).unwrap_err(),
            backend(expected)
        );
    }

    fn cleaned_image(count: usize) -> Vec<u8> {
        let (mut module, isa) = module();
        let signature = signature(&module);
        let function = module
            .declare_function("cleaned", Linkage::Export, &signature)
            .unwrap();
        let mut context = module.make_context();
        context.func.signature = signature;
        let mut scratch = FunctionBuilderContext::new();
        {
            let mut b = FunctionBuilder::new(&mut context.func, &mut scratch);
            let entry = b.create_block();
            let selected = b.create_block();
            let other = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.append_block_param(selected, types::I64);
            b.switch_to_block(entry);
            let input = b.block_params(entry)[0];
            let one = b.ins().iconst(types::I8, 1);
            b.ins().brif(one, selected, &[input.into()], other, &[]);
            b.switch_to_block(selected);
            let value = b.block_params(selected)[0];
            let result = b.ins().iadd_imm(value, 1);
            b.ins().return_(&[result]);
            b.switch_to_block(other);
            let result = b.ins().iconst(types::I64, 99);
            b.ins().return_(&[result]);
            b.seal_all_blocks();
            b.finalize();
        }
        fn clean(context: &mut Context, isa: &dyn TargetIsa) -> Result<String, NativeCompileError> {
            context.compute_cfg();
            context.compute_domtree();
            context.remove_constant_phis(isa).map_err(backend)?;
            context.verify(isa).map_err(backend)?;
            Ok(context.func.display().to_string())
        }
        let mut reference = Context::for_function(context.func.clone());
        let expected = clean(&mut reference, isa.as_ref()).unwrap();
        if count == 0 {
            clean(&mut context, isa.as_ref()).unwrap();
            module.define_function(function, &mut context).unwrap();
        } else {
            let pool = Pool::new(isa, count, DirectRuntimeCalls::default()).unwrap();
            let ticket = pool
                .submit_work(move |isa| {
                    let ir = clean(&mut context, isa)?;
                    let context = compile(context, isa)?;
                    Ok((context, ir))
                })
                .unwrap();
            let (context, actual) = ticket.wait().unwrap();
            assert_eq!(actual, expected);
            publish(&mut module, function, &context).unwrap();
        }
        module.finish().emit().unwrap()
    }

    #[test]
    fn owned_cleanup_and_machine_jobs_match_coordinator_ir_and_object() {
        let expected = cleaned_image(0);
        for count in [1, 2, 4] {
            for _ in 0..4 {
                assert_eq!(cleaned_image(count), expected);
            }
        }
    }

    #[test]
    fn typed_cleanup_errors_remain_ordered_by_their_tickets() {
        let (_, isa) = module();
        let pool = Pool::new(isa, 2, DirectRuntimeCalls::default()).unwrap();
        let first = pool
            .submit_work::<()>(|_| Err(backend("earlier cleanup")))
            .unwrap();
        let second = pool
            .submit_work::<()>(|_| Err(backend("later cleanup")))
            .unwrap();
        assert_eq!(first.wait().unwrap_err(), backend("earlier cleanup"));
        assert_eq!(second.wait().unwrap_err(), backend("later cleanup"));
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        if std::env::var_os("MECH_PROFILE_COMPILE_JOBS").is_some() {
            eprintln!(
                "compact-compile-jobs: workers={} submitted={} cleanup={}",
                self.workers.len(),
                self.submitted.get(),
                self.cleanup_submitted.get()
            );
        }
        self.queue.take();
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

impl<T> Ticket<T> {
    pub(super) fn wait(self) -> Result<T, NativeCompileError> {
        self.0.recv().map_err(backend)?
    }
}

pub(super) fn compile(
    mut context: Context,
    isa: &dyn TargetIsa,
) -> Result<Context, NativeCompileError> {
    // Retain the original ModuleError conversion and backend error spelling.
    let result: ModuleResult<()> = context
        .compile(isa, &mut ControlPlane::default())
        .map(|_| ())
        .map_err(Into::into);
    result.map_err(backend)?;
    Ok(context)
}

pub(super) fn publish<M: Module>(
    module: &mut M,
    function: FuncId,
    context: &Context,
) -> Result<(), NativeCompileError> {
    let code = context
        .compiled_code()
        .ok_or_else(|| backend("compiled job lacks code"))?;
    let relocs = code
        .buffer
        .relocs()
        .iter()
        .map(|reloc| ModuleReloc::from_mach_reloc(reloc, &context.func, function))
        .collect::<Vec<_>>();
    module
        .define_function_bytes(
            function,
            code.buffer.alignment as u64,
            code.buffer.data(),
            &relocs,
        )
        .map_err(backend)
}

enum Event {
    Function(FuncId, String, Linkage, Signature),
    AnonymousFunction(FuncId, Signature),
    Data(DataId, String, Linkage, bool, bool),
    AnonymousData(DataId, bool, bool),
    DefineFunction(FuncId, u64, Vec<u8>, Vec<ModuleReloc>),
    DefineData(DataId, DataDescription),
}

/// Prepare a later variant without publishing any events ahead of an earlier
/// function. Local IDs mirror the coordinator's declarations; replay checks them.
pub(super) struct RecordedModule {
    isa: OwnedTargetIsa,
    declarations: ModuleDeclarations,
    events: Vec<Event>,
    direct_runtime_calls: DirectRuntimeCalls,
}

impl RecordedModule {
    fn new(
        source: &ModuleDeclarations,
        isa: OwnedTargetIsa,
        direct_runtime_calls: DirectRuntimeCalls,
    ) -> ModuleResult<Self> {
        let mut declarations = ModuleDeclarations::default();
        for (expected, declaration) in source.get_functions() {
            let actual = if let Some(name) = &declaration.name {
                declarations
                    .declare_function(name, declaration.linkage, &declaration.signature)?
                    .0
            } else {
                declarations.declare_anonymous_function(&declaration.signature)?
            };
            assert_eq!(actual, expected);
        }
        for (expected, declaration) in source.get_data_objects() {
            let actual = if let Some(name) = &declaration.name {
                declarations
                    .declare_data(
                        name,
                        declaration.linkage,
                        declaration.writable,
                        declaration.tls,
                    )?
                    .0
            } else {
                declarations.declare_anonymous_data(declaration.writable, declaration.tls)?
            };
            assert_eq!(actual, expected);
        }
        Ok(Self {
            isa,
            declarations,
            events: Vec::new(),
            direct_runtime_calls,
        })
    }

    pub(super) fn replay<M: Module>(self, module: &mut M) -> Result<(), NativeCompileError> {
        for event in self.events {
            match event {
                Event::Function(expected, name, linkage, signature) => {
                    let actual = module
                        .declare_function(&name, linkage, &signature)
                        .map_err(backend)?;
                    assert_eq!(actual, expected);
                }
                Event::AnonymousFunction(expected, signature) => {
                    let actual = module
                        .declare_anonymous_function(&signature)
                        .map_err(backend)?;
                    assert_eq!(actual, expected);
                }
                Event::Data(expected, name, linkage, writable, tls) => {
                    let actual = module
                        .declare_data(&name, linkage, writable, tls)
                        .map_err(backend)?;
                    assert_eq!(actual, expected);
                }
                Event::AnonymousData(expected, writable, tls) => {
                    let actual = module
                        .declare_anonymous_data(writable, tls)
                        .map_err(backend)?;
                    assert_eq!(actual, expected);
                }
                Event::DefineFunction(id, alignment, bytes, relocs) => {
                    module
                        .define_function_bytes(id, alignment, &bytes, &relocs)
                        .map_err(backend)?;
                }
                Event::DefineData(id, description) => {
                    module.define_data(id, &description).map_err(backend)?;
                }
            }
        }
        Ok(())
    }
}

impl Module for RecordedModule {
    fn declare_func_in_func(
        &mut self,
        id: FuncId,
        f: &mut cranelift_codegen::ir::Function,
    ) -> cranelift_codegen::ir::FuncRef {
        self.direct_runtime_calls
            .declare_in(&self.declarations, id, f)
    }

    fn isa(&self) -> &dyn cranelift_codegen::isa::TargetIsa {
        self.isa.as_ref()
    }
    fn declarations(&self) -> &ModuleDeclarations {
        &self.declarations
    }

    fn declare_function(
        &mut self,
        name: &str,
        linkage: Linkage,
        signature: &Signature,
    ) -> ModuleResult<FuncId> {
        let id = self
            .declarations
            .declare_function(name, linkage, signature)?
            .0;
        self.events.push(Event::Function(
            id,
            name.to_owned(),
            linkage,
            signature.clone(),
        ));
        Ok(id)
    }
    fn declare_anonymous_function(&mut self, signature: &Signature) -> ModuleResult<FuncId> {
        let id = self.declarations.declare_anonymous_function(signature)?;
        self.events
            .push(Event::AnonymousFunction(id, signature.clone()));
        Ok(id)
    }
    fn declare_data(
        &mut self,
        name: &str,
        linkage: Linkage,
        writable: bool,
        tls: bool,
    ) -> ModuleResult<DataId> {
        let id = self
            .declarations
            .declare_data(name, linkage, writable, tls)?
            .0;
        self.events
            .push(Event::Data(id, name.to_owned(), linkage, writable, tls));
        Ok(id)
    }
    fn declare_anonymous_data(&mut self, writable: bool, tls: bool) -> ModuleResult<DataId> {
        let id = self.declarations.declare_anonymous_data(writable, tls)?;
        self.events.push(Event::AnonymousData(id, writable, tls));
        Ok(id)
    }
    fn define_function_with_control_plane(
        &mut self,
        id: FuncId,
        context: &mut Context,
        control: &mut ControlPlane,
    ) -> ModuleResult<()> {
        let code = context.compile(self.isa.as_ref(), control)?;
        let alignment = code.buffer.alignment as u64;
        let relocs = context
            .compiled_code()
            .unwrap()
            .buffer
            .relocs()
            .iter()
            .map(|reloc| ModuleReloc::from_mach_reloc(reloc, &context.func, id))
            .collect::<Vec<_>>();
        let bytes = context.compiled_code().unwrap().buffer.data().to_vec();
        self.events
            .push(Event::DefineFunction(id, alignment, bytes, relocs));
        Ok(())
    }
    fn define_function_bytes(
        &mut self,
        id: FuncId,
        alignment: u64,
        bytes: &[u8],
        relocs: &[ModuleReloc],
    ) -> ModuleResult<()> {
        self.events.push(Event::DefineFunction(
            id,
            alignment,
            bytes.to_vec(),
            relocs.to_vec(),
        ));
        Ok(())
    }
    fn define_data(&mut self, id: DataId, description: &DataDescription) -> ModuleResult<()> {
        self.events.push(Event::DefineData(id, description.clone()));
        Ok(())
    }
}
