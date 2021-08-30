// ----------------------------------------------------------------------------------------------------------
        // Planner
        // ----------------------------------------------------------------------------------------------------------
        // This is the start of a new planner. This will evolve into its own thing I imagine. It's messy and rough now
        for transformation_node in children {
          let constraint_text = formatter.format(&transformation_node, false);
          let mut compiled_tfm = self.compile_transformation(&transformation_node);
          let mut produces: HashSet<u64> = HashSet::new();
          let mut consumes: HashSet<u64> = HashSet::new();
          let this_one = compiled_tfm.clone();
          for transformation in compiled_tfm {
            match &transformation {
              Transformation::TableAlias{table_id, alias} => {
                produces.insert(*alias);
                match aliases.insert(*alias) {
                  true => (),
                  false => {
                    // This alias has already been marked as produced, so it is a duplicate
                    block.errors.insert(Error {
                      block_id: block.id,
                      step_text: constraint_text.clone(),
                      error_type: ErrorType::DuplicateAlias(*alias),
                    });
                  },
                }
              }
              Transformation::TableReference{table_id, reference} => {
                match table_id {
                  TableId::Local(id) => {
                    produces.insert(*id);
                  },
                  _ => (),
                };
              }
              Transformation::Whenever{table_id, ..} => {
                produces.insert(hash_string("~"));
              }
              Transformation::Constant{table_id, ..} => {
                match table_id {
                  TableId::Local(id) => {
                    produces.insert(*id);
                  },
                  _ => (),
                };
              }
              Transformation::NewTable{table_id, ..} => {
                match table_id {
                  TableId::Local(id) => {
                    produces.insert(*id);
                  },
                  _ => (),
                };
              },
              Transformation::Select{table_id, row, column, indices, out} => {
                match table_id {
                  TableId::Local(id) => {consumes.insert(*id);},
                  _ => (),
                }
                for (row, column) in indices {
                  match row {
                    TableIndex::Table(TableId::Local(id)) => {consumes.insert(*id);},
                    _ => (),
                  }
                  match column {
                    TableIndex::Table(TableId::Local(id)) => {consumes.insert(*id);},
                    _ => (),
                  }
                }
                match out {
                  TableId::Local(id) => {
                    produces.insert(*id);
                  },
                  _ => (),
                };
              },
              Transformation::Function{name, arguments, out} => {
                for (_, table_id, row, column) in arguments {
                  match row {
                    TableIndex::Table(TableId::Local(id)) => {consumes.insert(*id);},
                    _ => (),
                  }
                  match table_id {
                    TableId::Local(id) => {consumes.insert(*id);},
                    _ => (),
                  }
                }
                let (out_id, row, column) = out;
                match out_id {
                  TableId::Local(id) => {produces.insert(*id);},
                  _ => (),
                }
                match row {
                  TableIndex::Table(TableId::Local(id)) => {
                    consumes.insert(*id);
                  }
                  _ => (),
                }
                match column {
                  TableIndex::Table(TableId::Local(id)) => {
                    consumes.insert(*id);
                  }
                  _ => (),
                }
              }
              _ => (),
            }
            transformations.push(transformation.clone());
          }
          //transformations.append(&mut functions);
          // If the constraint doesn't consume anything, put it on the top of the plan. It can run any time.
          if consumes.len() == 0 || consumes.difference(&produces).cloned().collect::<HashSet<u64>>().len() == 0 {
            block_produced = block_produced.union(&produces).cloned().collect();
            plan.insert(0, (constraint_text, produces, consumes, this_one));
          // Otherwise, the constraint consumes something, and we have to see if it's satisfied
          } else {
            let mut satisfied = false;
            //let (step_node, step_produces, step_consumes, step_constraints) = step;
            let union: HashSet<u64> = block_produced.union(&produces).cloned().collect();
            let unsatisfied: HashSet<u64> = consumes.difference(&union).cloned().collect();
            if unsatisfied.is_empty() {
              block_produced = block_produced.union(&produces).cloned().collect();
              plan.push((constraint_text, produces, consumes, this_one));
            } else {
              unsatisfied_transformations.push((constraint_text, produces, consumes, this_one));
            }
          }

          // Check if any of the unsatisfied constraints have been met yet. If they have, put them on the plan.
          let mut now_satisfied = unsatisfied_transformations.drain_filter(|unsatisfied_transformation| {
            let (text, unsatisfied_produces, unsatisfied_consumes, _) = unsatisfied_transformation;
            let union: HashSet<u64> = block_produced.union(&unsatisfied_produces).cloned().collect();
            let unsatisfied: HashSet<u64> = unsatisfied_consumes.difference(&union).cloned().collect();
            match unsatisfied.is_empty() {
              true => {
                block_produced = block_produced.union(&unsatisfied_produces).cloned().collect();
                true
              }
              false => false
            }
          }).collect::<Vec<_>>();
          plan.append(&mut now_satisfied);
        }
        // Do a final check on unsatisfied constraints that are now satisfied
        let mut now_satisfied = unsatisfied_transformations.drain_filter(|unsatisfied_transformation| {
          let (_, unsatisfied_produces, unsatisfied_consumes, _) = unsatisfied_transformation;
          let union: HashSet<u64> = block_produced.union(&unsatisfied_produces).cloned().collect();
          let unsatisfied: HashSet<u64> = unsatisfied_consumes.difference(&union).cloned().collect();
          match unsatisfied.is_empty() {
            true => {
              block_produced = block_produced.union(&unsatisfied_produces).cloned().collect();
              true
            }
            false => false
          }
        }).collect::<Vec<_>>();

        plan.append(&mut now_satisfied);
        
        let mut global_out = vec![];
        let mut block_transformations = vec![];
        let mut to_copy: HashMap<TableId, Vec<Transformation>> = HashMap::new();
        let mut new_steps = vec![];
        for step in plan {
          let (step_text, _, _, mut step_transformations) = step;
          let mut rtfms = step_transformations.clone();
          rtfms.reverse();
          for tfm in rtfms {
            match tfm {
              Transformation::TableReference{table_id, reference} => {
                let referenced_table_id = reference.as_reference().unwrap();
                block.plan.push(Transformation::Function{
                  name: *TABLE_COPY,
                  arguments: vec![(0,TableId::Local(referenced_table_id), TableIndex::All, TableIndex::All)],
                  out: (TableId::Global(referenced_table_id), TableIndex::All, TableIndex::All),
                });
                match to_copy.get(&TableId::Local(referenced_table_id)) {
                  Some(aliases) => {
                    for alias in aliases {
                      match alias {
                        Transformation::ColumnAlias{table_id, column_ix, column_alias} => {
                          new_steps.push(Transformation::ColumnAlias{table_id: TableId::Global(*table_id.unwrap()), column_ix: *column_ix, column_alias: *column_alias});                          
                        }
                        _ => (),
                      }
                    }
                  }
                  _ => (),
                }
              }
              Transformation::Whenever{..} => {
                block.plan.push(tfm.clone());
              }
              Transformation::Select{..} => {
                block.plan.push(tfm.clone());
              }
              Transformation::ColumnAlias{table_id, column_ix, column_alias} => {
                let aliases = to_copy.entry(table_id).or_insert(Vec::new());
                aliases.push(tfm.clone());
              }
              Transformation::Function{name, ref arguments, out} => {
                let (out_id, row, column) = out;
                match out_id {
                  TableId::Local(id) => block.plan.push(tfm.clone()),
                  _ => {
                    global_out.push(tfm.clone());
                  },
                }
              }
              _ => (),
            }
          }
          step_transformations.append(&mut new_steps);
          block_transformations.push((step_text, step_transformations));
        }
        block.plan.append(&mut global_out);

        // Here we try to optimize the plan a little bit. The compiler will generate chains of concatenating
        // tables sometimes where there is no actual work to be done. If we remove these moot intermediate steps,
        // we can save time. We do this by comparing the input and outputs of consecutive steps. If the two steps
        // can be condensed into one step, we do this.
        let mut defunct_tables = vec![];
        if block.plan.len() > 1 {
          let mut new_plan = vec![];
          let mut step_ix = 0;
          loop {
            if step_ix >= block.plan.len() - 1 {
              if step_ix == block.plan.len() - 1 {
                new_plan.push(block.plan[step_ix].clone());
              }
              break;
            }
            let this = &block.plan[step_ix];
            let next = &block.plan[step_ix + 1];
            match (this, next) {
              (Transformation::Function{name, arguments, out}, Transformation::Function{name: name2, arguments: arguments2, out: out2}) => {
                if (*name2 == *TABLE_HORZCAT || *name2 == *TABLE_VERTCAT) && arguments2.len() == 1 {
                  defunct_tables.append(&mut arguments2.clone());
                  let (_, out_table2, out_row2, out_column2) = arguments2[0];
                  if *out == (out_table2, out_row2, out_column2) {
                    let new_step = Transformation::Function{name: *name, arguments: arguments.clone(), out: *out2};
                    new_plan.push(new_step);
                    step_ix += 2;
                    continue;
                  }
                }
                new_plan.push(this.clone());
              }
              (Transformation::Select{table_id, row, column, indices, out}, _) => {
                if indices.len() == 1 {
                  defunct_tables.push((0, *table_id, TableIndex::None, TableIndex::None));
                } else {
                  new_plan.push(this.clone());
                }
              }
              _ => new_plan.push(this.clone()),
            }
            step_ix += 1;
          }
          block.plan = new_plan;

          // Combine steps with set, e.g.:
          // 1. math/add(#ball{:,:}, moe-veg-cut-six{:,:}) -> yel-ohi-sod-fil{:,:}
          // 2. table/set(yel-ohi-sod-fil{:,:}) -> #ball{:,:}
          // Becomes
          // 1. math/add(#ball{:,:}, moe-veg-cut-six{:,:}) -> #ball{:,:}
          let mut include = vec![];
          let mut exclude: HashSet<Transformation> = HashSet::new();
          'step_loop: for step in &block.plan {
            match step {
              Transformation::Function{name, arguments, out} => {
                for step2 in &block.plan {
                  match step2 {
                    Transformation::Function{name: name2, arguments: arguments2, out: out2} => {
                      if *name2 == *TABLE_SET  && arguments2.len() == 1 {
                        let (_, table, row, column) = arguments2[0];
                        if (table,row,column) == *out && row == TableIndex::All && column == TableIndex::All {
                          include.push(Transformation::Function{name: *name, arguments: arguments.clone(), out: *out2});
                          exclude.insert(step2.clone());
                          exclude.insert(step.clone());
                          continue 'step_loop;
                        }
                      }
                    }
                    _ => (),
                  }
                }
                match exclude.contains(&step) {
                  false => include.push(step.clone()),
                  _ => (),
                }
              }
              _ => {
                match exclude.contains(&step) {
                  false => include.push(step.clone()),
                  _ => (),
                }
              },
            }
          }
          //block.plan = include;
        }
        /*// Remove all defunct tables from the transformation list. These would be tables that were written to by
        // some function that was removed from the previous optimization pass
        let defunct_table_ids = defunct_tables.iter().map(|(_, table_id, _, _)| table_id).collect::<HashSet<&TableId>>();
        let mut new_transformations = vec![];
        for (tfm_text, steps) in &block_transformations {
          let mut new_steps = vec![];
          for step in steps {
            match step {
              Transformation::NewTable{table_id, ..} => {
                match defunct_table_ids.contains(&table_id) {
                  true => continue,
                  false => new_steps.push(step.clone()),
                }
              }
              _ => new_steps.push(step.clone()),
            }
          }
          new_transformations.push((tfm_text.clone(), new_steps));
        }
        block_transformations = new_transformations;*/
        // End Planner ----------------------------------------------------------------------------------------------------------
