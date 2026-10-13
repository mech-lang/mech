import std/[os, typedthreads]
createDir(getEnv("MECH_NIM_TREE_TRACE"))
proc auditOpen*(label: string): File =
  result = open(getEnv("MECH_NIM_TREE_TRACE") & "/" & label & "-" & $getThreadId() & ".trace", fmAppend)
  result.writeLine("BEGIN|", label, "|", getThreadId())
