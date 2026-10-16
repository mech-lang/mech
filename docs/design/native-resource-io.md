# Native resource I/O

Native programs read host inputs and send outputs through ordinary Mech context
bindings. Numeric parsing and formatting remain pure library operations.

```mech
+> @args := cli/args
+> @out := cli/stdout

n := number/parse-u64(@args/1)
@out/line <- number/format(n, 0)
```

The CLI argument context exposes the first user argument at `@args/1`, excluding
the executable name. It returns Unicode text. Missing arguments, invalid Unicode
and noncanonical argument paths are errors. The stdout context supports `/text`
for exact text and `/line` for text followed by a newline.

## Compilation and activation

The native compiler lowers canonical context imports, resource paths and sends
into typed resource operations. It derives an immutable list of exact provider
identities, paths and intents from those operations. The generated application
embeds that list with its constant table and Cranelift object.

Before executing the object, the host validates and binds every requested port.
Binding creates no authority and observes no inputs. Source declarations cannot
grant themselves access. Activated execution rejects legacy process I/O and
unbound resource identities, including within pure callback scopes. Failed
activated admission does not select a legacy native backend.

The current native resource subset supports snapshot CLI arguments and stdout
text sends. Live input, context assignment, host I/O within callbacks and
arbitrary configured host instances require further compiler support. Official
artifact admission and resident scheduling remain separate integration work.

## Launch policies

One executable supports two explicit launch policies. The arithmetic object,
constants, resource requests and CLI provider stay identical.

* **Secure**, the default, constructs the ordinary runtime and capability kernel.
  The launcher supplies scoped grants. Every resource use checks authority. One
  explicit execution transaction stages sends, commits after computation and
  deferred work succeed, and aborts on failure. Runtime shutdown is part of
  completion.
* **Trusted** skips the capability kernel and runtime transactions. It retains
  the same provider's path, direction, payload, newline and delivery behavior.
  This is a host-selected execution policy for trusted programs.

`MECH_NATIVE_SECURITY` selects `secure` or `trusted` at launch. Secure execution
reads `MECH_NATIVE_GRANTS`, a path to a JSON array supplied by the launcher. An
absent grant file supplies no authority. For the example, the grants are:

```json
[
  {"target": "cli/args", "operations": ["read"], "paths": ["1"]},
  {"target": "cli/stdout", "operations": ["write"], "paths": ["line"]}
]
```

Both launches use the same compiled file:

```sh
MECH_NATIVE_SECURITY=secure MECH_NATIVE_GRANTS=grants.json ./app 5500
MECH_NATIVE_SECURITY=trusted ./app 5500
```

## Qualification and measurement

The native I/O tests cover exact binding, denied access before observation,
revocation, transaction commit and abort, ordinary text concatenation, pure
numeric parsing, and successful output through native and resident sequence
execution. An explicit generated-application test links once and runs that
executable under both policies, checks denied launches, and pins the executable,
object, constant and requirement hashes.

Comparisons between these policies must measure complete process lifetime,
including runtime construction, grant loading, port binding, input, computation,
output delivery and shutdown. Both modes use the same executable hash and input.
Compiler and browser work must stop during timing collection. Existing timings
retain their original source and artifact provenance; revised source programs
require their own qualification and measurements.
