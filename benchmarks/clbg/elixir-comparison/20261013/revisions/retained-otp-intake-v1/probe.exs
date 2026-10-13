# Toolchain metadata only; no Clbg module is compiled, loaded or called.
for {key, value} <- [
  {"ELIXIR", System.version()},
  {"OTP", List.to_string(:erlang.system_info(:otp_release))},
  {"ERTS", List.to_string(:erlang.system_info(:version))},
  {"ARCH", List.to_string(:erlang.system_info(:system_architecture))},
  {"EMU_FLAVOR", Atom.to_string(:erlang.system_info(:emu_flavor))},
  {"SCHEDULERS", Integer.to_string(:erlang.system_info(:schedulers))},
  {"SCHEDULERS_ONLINE", Integer.to_string(:erlang.system_info(:schedulers_online))},
  {"DIRTY_CPU", Integer.to_string(:erlang.system_info(:dirty_cpu_schedulers))},
  {"DIRTY_CPU_ONLINE", Integer.to_string(:erlang.system_info(:dirty_cpu_schedulers_online))},
  {"DIRTY_IO", Integer.to_string(:erlang.system_info(:dirty_io_schedulers))},
  {"ASYNC_THREADS", Integer.to_string(:erlang.system_info(:thread_pool_size))},
  {"BOOTSTRAP_PROCESS_COUNT", Integer.to_string(:erlang.system_info(:process_count))},
  {"CLBG_MAIN_LOADED", inspect(:code.is_loaded(Clbg.Main))},
  {"CLBG_SUPPORT_LOADED", inspect(:code.is_loaded(Clbg.Support))},
  {"GC_DEFAULTS_TEXT64", Base.encode64(inspect(:erlang.system_info(:garbage_collection)))},
  {"ELIXIR_BEAM64", Base.encode64(List.to_string(:code.which(:elixir)))},
  {"SYSTEM_BEAM64", Base.encode64(List.to_string(:code.which(Elixir.System)))}
] do
  IO.puts(key <> "=" <> value)
end
for path <- :code.get_path(), do: IO.puts("CODE_PATH64=" <> Base.encode64(List.to_string(path)))
