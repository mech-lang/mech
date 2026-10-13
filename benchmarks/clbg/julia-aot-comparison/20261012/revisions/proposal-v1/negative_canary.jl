# Source-only, unscored negative control. This is never a CLBG source replacement.
# The eventual image receipt must prove missing_native has no native or generic
# native invoke before execution; compile=min intent alone cannot establish that.
module ClbgAotNegativeCanary
module MissingNative
    Base.Experimental.@compiler_options compile=min
    mutable struct Payload
        value::Int
    end
    Base.@noinline function missing_native(payload::Payload)
        payload.value += 1
        return payload.value
    end
end

# Build this exported entry as native code, while deliberately withholding the
# target method's native coverage. Keep its IR for the interpreter canary;
# an independently preserved stripped target image exercises MissingCodeError.
Base.@ccallable function clbg_entry(argc::Cint, argv::Ptr{Ptr{Cchar}})::Cint
    try
        argc == 1 || return Cint(64)
        value = parse(Int, unsafe_string(unsafe_load(argv, 1)))
        payload = MissingNative.Payload(value)
        result = Base.invokelatest(MissingNative.missing_native, payload)
        println(result)
        # This can intentionally exit0 after interpreting. The qualification
        # oracle must still reject it even if trace-compile is empty.
        return Cint(0)
    catch exception
        showerror(stderr, exception)
        println(stderr)
        return Cint(1)
    end
end
end
