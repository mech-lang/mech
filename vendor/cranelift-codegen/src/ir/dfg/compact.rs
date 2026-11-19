//! Release retired value IDs after the producer has discarded its SSA handles.
use super::*;

impl DataFlowGraph {
    /// Resolve aliases and densely renumber the remaining values, in their
    /// original order. Returns the number of retired value IDs removed.
    ///
    /// This invalidates ALL external `Value` handles. Call only after the IR
    /// producer has finished and before creating value-indexed analysis state.
    /// Instructions, blocks, layout, types and effects retain their identities.
    pub fn compact_values(&mut self) -> usize {
        let before = self.num_values();
        // BlockArg reserves two tag bits. The temporary namespace below must
        // fit its payload too. Huge functions retain the ordinary path.
        if before >= (1 << 29) {
            return 0;
        }
        self.resolve_alias_chains();
        let retained =
            |data: ValueDataPacked| !matches!(ValueData::from(data), ValueData::Alias { .. });
        let count = self.values.values().filter(|&&data| retained(data)).count();
        if count == before {
            return 0;
        }
        let mut values = PrimaryMap::<Value, ValueDataPacked>::with_capacity(count);
        let mut mapping = PrimaryMap::<Value, Value>::with_capacity(before);
        for &data in self.values.values() {
            mapping.push(if retained(data) {
                values.push(data)
            } else {
                Value::reserved_value()
            });
        }
        // Resolve aliases in the mapping before retiring them. Unlike the
        // ordinary alias pass, storage compaction must also rewrite aliases in
        // unreferenced jump/exception tables and union inputs.
        for (alias, &data) in self.values.iter() {
            if let ValueData::Alias { original, .. } = ValueData::from(data) {
                if original != Value::reserved_value() {
                    mapping[alias] = mapping[original];
                }
            }
        }
        // Pooled argument lists may be shared (e.g. copied BlockCalls). Map
        // through a disjoint temporary namespace so each sweep is idempotent:
        // revisiting the same storage must never renumber it a second time.
        self.remap_value_storage(|value| {
            if (value.as_u32() as usize) < before {
                let mapped = mapping[value];
                assert_ne!(mapped, Value::reserved_value(), "use of retired value");
                Value::from_u32(before as u32 + mapped.as_u32())
            } else {
                value
            }
        });
        self.remap_value_storage(|value| {
            if value.as_u32() as usize >= before {
                Value::from_u32(value.as_u32() - before as u32)
            } else {
                value
            }
        });
        // Union nodes and debug-label assignments are not pooled or shared.
        for data in values.values_mut() {
            if let ValueData::Union { ty, x, y } = ValueData::from(*data) {
                *data = ValueData::Union {
                    ty,
                    x: mapping[x],
                    y: mapping[y],
                }
                .into();
            }
        }
        if let Some(labels) = &mut self.values_labels {
            // Match resolve_all_aliases' best-effort debug-label policy.
            labels.retain(|&value, _| {
                !matches!(ValueData::from(self.values[value]), ValueData::Alias { .. })
            });
            *labels = mem::take(labels)
                .into_iter()
                .map(|(value, mut label)| {
                    if let ValueLabelAssignments::Alias { value, .. } = &mut label {
                        *value = mapping[*value];
                    }
                    (mapping[value], label)
                })
                .collect();
        }
        #[cfg(feature = "construction-census")]
        {
            // The replacement table coexists with the old arena until this
            // assignment. The temporary Value-to-Value map is scratch storage,
            // not an SSA arena, and is deliberately outside these counters.
            census::allocate(Arena::Values, count, count);
            census::release(Arena::Values, before);
        }
        self.values = values;
        before - count
    }

    /// `map` must be idempotent because pooled argument storage may be shared.
    fn remap_value_storage(&mut self, map: impl Fn(Value) -> Value + Copy) {
        for inst in self.insts.0.values_mut() {
            inst.map_values(
                &mut self.value_lists,
                &mut self.jump_tables,
                &mut self.exception_tables,
                map,
            );
        }
        // Include unreferenced side tables as well as those reached above.
        for table in self.jump_tables.values_mut() {
            for branch in table.all_branches_mut() {
                branch.update_args(&mut self.value_lists, |arg| arg.map_value(map));
            }
        }
        for table in self.exception_tables.values_mut() {
            for branch in table.all_branches_mut() {
                branch.update_args(&mut self.value_lists, |arg| arg.map_value(map));
            }
            for context in table.contexts_mut() {
                *context = map(*context);
            }
        }
        for results in self.results.values_mut() {
            for value in results.as_mut_slice(&mut self.value_lists) {
                *value = map(*value);
            }
        }
        for block in self.blocks.0.values_mut() {
            for value in block.params.as_mut_slice(&mut self.value_lists) {
                *value = map(*value);
            }
        }
    }
}
