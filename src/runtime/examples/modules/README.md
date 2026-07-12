# Runtime module smoke example

`main.mec` imports `math.mec` as module namespace `math`.

`math.mec` defines and exports `tau`.

`main.mec` accesses the exported value as `math/tau`.

Import forms:

~~~text
+> ./math.mec    exposes exports as math/<name>
+> math          exposes exports as math/<name>
+> math/tau      exposes tau unqualified
+> math/*        exposes all exported names unqualified
<+ tau           exports tau
~~~

Modules execute in isolated environments.

Only declared exports are visible to importers.

Non-exported symbols are private.

Conflicting imports fail.
