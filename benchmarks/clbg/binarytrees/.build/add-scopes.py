from pathlib import Path
p=Path('src/engine/src/native_sequence/control.rs');s=p.read_text()
s=s.replace('    Print {\n','    Arena {\n        id: usize,\n        capacity: Option<Box<Expr>>,\n        arity: usize,\n        function: String,\n        arguments: Box<Expr>,\n    },\n    Print {\n',1)
marker='            if name == "control/while" {'
insert='''            if name == "memory/with-ref-arena" || name == "memory/with-frame" {
                let region = name == "memory/with-ref-arena";
                if a.len() != if region { 4 } else { 2 } {
                    return Err("memory scope signature".into());
                }
                let capacity = if region { Some(Box::new(self.expr(&a[0], vars)?)) } else { None };
                let arity = if region {
                    let e = self.expr(&a[1], vars)?;
                    let Op::Integer(n) = e.op else { return Err("arena arity must be literal".into()); };
                    if !(1..=32).contains(&n) { return Err("arena arity 1..32".into()); }
                    n as usize
                } else { 0 };
                if capacity.as_ref().is_some_and(|e| e.kind != Kind::Integer) {
                    return Err("arena capacity must be u64".into());
                }
                let i = if region { 2 } else { 0 };
                let function = text(&a[i])?.trim().to_owned();
                let arguments = self.expr(&a[i + 1], vars)?;
                let f = self.function(&function)?;
                if f.inputs != flat(&arguments.kind)
                    || f.inputs.iter().any(|k| k != &Kind::Integer)
                    || flat(&f.output.kind).iter().any(|k| k != &Kind::Integer)
                {
                    return Err("memory scope callbacks require scalar u64 inputs and outputs; references cannot escape".into());
                }
                let id = self.next_effect;
                self.next_effect += 1;
                return Ok(Expr { kind: f.output.kind, op: Op::Arena { id, capacity, arity, function, arguments: Box::new(arguments) } });
            }
'''
assert marker in s;s=s.replace(marker,insert+marker,1)
s=s.replace('"heap/read-refs" | "heap/take-refs" if kinds', '"heap/read-refs" if kinds').replace('action: if name == "heap/read-refs" { 2 } else { 3 },','action: 2,')
a=s.index('                "heap/stats" if values.is_empty() => (');b=s.index('                "io/print"',a);s=s[:a]+s[b:]
s=s.replace('name.starts_with("heap/") || name == "io/print"','name.starts_with("heap/") || name.starts_with("memory/") || name == "io/print"')
s=s.replace('                Op::Heap { arguments, .. } => {','''                Op::Arena { capacity, function, arguments, .. } => {
                    allow_heap && visit_name(function, seen)
                        && capacity.as_ref().is_none_or(|e| closed(e, functions, seen, allow_heap))
                        && closed(arguments, functions, seen, allow_heap)
                }
                Op::Heap { arguments, .. } => {''',1)
s=s.replace('                Op::Field(e, _) => scheduling(e, functions)?,','''                Op::Arena { capacity, arguments, .. } => {
                    if let Some(e) = capacity { scheduling(e, functions)?; }
                    scheduling(arguments, functions)?;
                }
                Op::Field(e, _) => scheduling(e, functions)?,''')
p.write_text(s)
p=Path('src/stdlib/src/native_sequence.rs');s=p.read_text().replace('        "heap/take-refs",\n        "heap/stats",','        "memory/with-ref-arena",\n        "memory/with-frame",');p.write_text(s)
