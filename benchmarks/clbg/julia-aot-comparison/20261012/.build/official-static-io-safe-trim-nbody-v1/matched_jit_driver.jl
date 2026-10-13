include(joinpath(@__DIR__, "ClbgNbodyPilot", "src", "ClbgNbodyPilot.jl"))
ClbgNbodyPilot.initialize_runtime_state()
ClbgNbodyPilot.run_original_driver()
