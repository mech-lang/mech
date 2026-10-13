# Source-only proposal, included at build time inside each reviewed app module.
# run_original_driver() must contain the explicitly derived original driver and
# fresh runtime state construction. No original script is included at runtime.
# Each actual compiled authored file/span is counted once: included template,
# exact derived original bodies and C glue. Unused provenance copies are evidence.
Base.@ccallable function clbg_entry(argc::Cint, argv::Ptr{Ptr{Cchar}})::Cint
    try
        empty!(ARGS)
        for index in 1:Int(argc)
            push!(ARGS, unsafe_string(unsafe_load(argv, index)))
        end
        initialize_runtime_state()
        run_original_driver()
        return Cint(0)
    catch
        println(Core.stderr, "nbody entry failed")
        return Cint(1)
    end
end
