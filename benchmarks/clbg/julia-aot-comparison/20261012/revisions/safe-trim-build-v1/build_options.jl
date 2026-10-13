# Build-process instrumentation only; no workload or application entry call.
# The official builder and all three application source files stay byte-exact.
let builder = popfirst!(ARGS), options_output = pop!(ARGS)
    @assert length(ARGS) == 4
    app_source, output_kind, include_ccallable_roots, abi_output = copy(ARGS)
    @assert output_kind == "--output-lib" && include_ccallable_roots == "true"
    options = Base.JLOptions()
    @assert options.trim == 1 # JL_TRIM_SAFE in the pinned julia.h.
    @assert options.strip_ir == 1 && options.strip_metadata == 0
    @assert options.incremental == 0 && options.opt_level == 3 && options.debug_level == 2
    @assert options.outputo != C_NULL && options.outputbc != C_NULL && options.image_file != C_NULL
    @assert Threads.nthreads(:default) == 8
    actual = Dict{String,Any}(
        "trim" => Int(options.trim), "strip_ir" => Int(options.strip_ir),
        "strip_metadata" => Int(options.strip_metadata), "incremental" => Int(options.incremental),
        "opt_level" => Int(options.opt_level), "debug_level" => Int(options.debug_level),
        "default_threads" => Threads.nthreads(:default),
        "output_o" => unsafe_string(options.outputo), "output_bc" => unsafe_string(options.outputbc),
        "input_image" => unsafe_string(options.image_file), "app_source" => app_source,
        "output_kind" => output_kind, "include_ccallable_roots" => true, "abi_output" => abi_output,
        "workload_state_initialized" => false)
    Base.include(Main,builder)
    Core.@latestworld
    @assert isdefined(Main,:ClbgNbodyPilot)
    @assert !isdefined(Main.ClbgNbodyPilot,:bodies)
    @assert isempty(Base.ARGS) && isempty(Core.ARGS) && isempty(LOAD_PATH) && isempty(DEPOT_PATH)
    # This is emitted before the compiler's final safe verifier/code generation.
    # It alone is never a successful verification or runtime-AOT receipt.
    open(options_output,"w") do io
        Base.TOML.print(io,actual)
    end
end
