# Unscored compiled-source/stdio observation. No benchmark entry is called.
[output, ebin, main_source, support_source] = System.argv()
true = System.version() == "1.20.4"
true = :erlang.system_info(:otp_release) == ~c"29"
true = :erlang.system_info(:version) == ~c"17.1"
true = :erlang.system_info(:emu_flavor) == :jit
true = :erlang.system_info(:schedulers_online) == 8
false = :code.is_loaded(Clbg.Main)
false = :code.is_loaded(Clbg.Support)

records = for {module, source} <- [{Clbg.Main, main_source}, {Clbg.Support, support_source}] do
  beam = Path.join(ebin, Atom.to_string(module) <> ".beam")
  {:ok, {^module, chunks}} = :beam_lib.chunks(String.to_charlist(beam), [:compile_info, :debug_info, :exports])
  compile_info = Keyword.fetch!(chunks, :compile_info)
  true = Path.expand(List.to_string(Keyword.fetch!(compile_info, :source))) == Path.expand(source)
  {:debug_info_v1, :elixir_erl, {:elixir_v1, %{module: ^module, file: debug_source} = original_debug, _specs}} = Keyword.fetch!(chunks, :debug_info)
  true = is_binary(debug_source) and Path.expand(debug_source) == Path.expand(source)
  true = is_list(Map.fetch!(original_debug, :definitions)) and Map.fetch!(original_debug, :definitions) != []
  exports = Keyword.fetch!(chunks, :exports)
  true = Enum.member?(exports, if(module == Clbg.Main, do: {:main, 1}, else: {:write, 1}))
  {:module, ^module} = Code.ensure_loaded(module)
  true = Path.expand(List.to_string(:code.which(module))) == Path.expand(beam)
  record = %{module: module, beam: beam, original_source: source, compile_info: compile_info,
    debug_info: Keyword.fetch!(chunks, :debug_info), exports: exports, main_entry_executed: false,
    canonical_loc: nil, source_work_qualified: false}
  File.write!(Path.join(output, Atom.to_string(module) <> ".ownership.etf"), :erlang.term_to_binary(record), [:exclusive])
  module
end
true = records == [Clbg.Main, Clbg.Support]
<<>> = apply(Clbg.Support, :input, [])
payload = :binary.copy(for(value <- 0..255, into: <<>>, do: <<value>>), 513)
:ok = apply(Clbg.Support, :write, [payload])
File.write!(Path.join(output, "summary.txt"), "compiled_modules=2\nsource_debug_bindings=2\ninput_eof=true\nraw_output_bytes=131328\nmain_entry_executed=false\nnonempty_input_qualified=false\n", [:exclusive])
