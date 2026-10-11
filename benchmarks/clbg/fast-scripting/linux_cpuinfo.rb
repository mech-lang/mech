# Supply the standard Linux CPU enumeration probe on macOS; no algorithm changes.
module CLBGLinuxCPUInfo
  def read(path, *arguments, **keywords, &block)
    path = ENV.fetch('CLBG_CPUINFO_FILE') if path == '/proc/cpuinfo'
    super(path, *arguments, **keywords, &block)
  end
end
IO.singleton_class.prepend(CLBGLinuxCPUInfo)
