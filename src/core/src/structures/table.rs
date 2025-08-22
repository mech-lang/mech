use crate::*;
#[cfg(feature = "matrix")]
use crate::matrix::Matrix;
use indexmap::map::*;

// Table ------------------------------------------------------------------

#[cfg(feature = "table")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MechTable {
  pub rows: usize,
  pub cols: usize,
  pub data: IndexMap<u64,(ValueKind,Matrix<Value>)>,
  pub col_names: HashMap<u64,String>,
}

#[cfg(feature = "table")]
impl MechTable {

  pub fn from_records(records: Vec<MechRecord>) -> MResult<MechTable> {
    if records.is_empty() {
      return Err(MechError { id: line!(), file: file!().to_string(), tokens: vec![], msg: "Cannot create MechTable from empty record list.".to_string(), kind: MechErrorKind::None});
    }

    let first = &records[0];
    let rows = records.len();
    let cols = first.cols;

    let mut col_data: IndexMap<u64, Vec<Value>> = IndexMap::new();

    for (&col_id, value) in &first.data {
      col_data.insert(col_id, vec![value.clone()]);
    }

    let mut kinds = IndexMap::new();
    for (col_id, kind) in first.data.keys().zip(&first.kinds) {
      kinds.insert(*col_id, kind.clone());
    }

    let col_names = first.field_names.clone();

    for record in records.iter().skip(1) {
      first.check_record_schema(record)?;
      for (&col_id, value) in &record.data {
        col_data.entry(col_id).or_insert_with(Vec::new).push(value.clone());
      }
    }

    let data: IndexMap<u64, (ValueKind, Matrix<Value>)> = col_data
      .into_iter()
      .map(|(col_id, values)| {
        let kind = kinds[&col_id].clone();
        let matrix = Matrix::DVector(Ref::new(DVector::from_vec(values)));
        (col_id, (kind, matrix))
      })
      .collect();

    Ok(MechTable {rows,cols,data,col_names})
  }

  pub fn from_kind(kind: ValueKind) -> MResult<MechTable> {
    match kind {
      ValueKind::Table(tbl,sze) => {
        let mut data = IndexMap::new();
        let mut col_names = HashMap::new();
        for (col_id, col_kind) in &tbl {
          let matrix = Matrix::DVector(Ref::new(DVector::from_vec(vec![Value::Empty; sze])));
          col_names.insert(hash_str(col_id), col_id.clone());
          data.insert(hash_str(&col_id), (col_kind.clone(), matrix));
        }
        Ok(MechTable {rows: sze, cols: tbl.len(), data, col_names})
      }
      _ => {
        return Err(MechError { id: line!(), file: file!().to_string(), tokens: vec![], msg: "Cannot create MechTable from non-table kind.".to_string(), kind: MechErrorKind::None });
      }
    }
  }

  pub fn empty_table(&self, rows: usize) -> MechTable {
    let mut data = IndexMap::new();
    for col in self.data.iter() {
      let (key, (kind, matrix)) = col;
      // make a new vector the length of ix with values Value::Empty
      let elements = vec![Value::Empty; rows];
      let new_matrix = Matrix::DVector(Ref::new(DVector::from_vec(elements)));
      data.insert(*key, (kind.clone(), new_matrix));
    }
    MechTable { rows: rows, cols: self.cols, data, col_names: self.col_names.clone() }
  }

  pub fn check_record_schema(&self, record: &MechRecord) -> MResult<bool> {

    for (&col_id, record_value) in &record.data {
      // Check that the column exists in the table
      // self.get data col id _or continue to the next column
      let (expected_kind, column_matrix) = match self.data.get(&col_id) {
        Some(data) => data,
        None => {
          continue;
        }
      };

      // Check actual value kind
      let actual_kind = record_value.kind();

      if expected_kind != &actual_kind {
        return Err(MechError {id: line!(),file: file!().to_string(),tokens: vec![],msg: format!("Schema mismatch: column {} kind mismatch (expected: {:?}, found: {:?})",col_id, expected_kind, actual_kind),kind: MechErrorKind::None,});
      }

      // Check column name
      if let Some(expected_name) = self.col_names.get(&col_id) {
        if let Some(field_name) = record.field_names.get(&col_id) {
          if expected_name != field_name {
            return Err(MechError {id: line!(),file: file!().to_string(),tokens: vec![],msg: format!("Schema mismatch: column {} name mismatch (expected: '{}', found: '{}')",col_id, expected_name, field_name),kind: MechErrorKind::None,});
          }
        }
      }
    }

    Ok(true)
  }
  
  pub fn check_table_schema(&self, record: &MechTable) -> MResult<bool> {

    // Check that the column names match
    for (&col_id, col_name) in &self.col_names {
      if let Some(record_name) = record.col_names.get(&col_id) {
        if col_name != record_name {
          return Err(MechError {id: line!(),file: file!().to_string(),tokens: vec![],msg: format!("Schema mismatch: column {} name mismatch (expected: '{}', found: '{}')",col_id, col_name, record_name),kind: MechErrorKind::None,});
        }
      } else {
        return Err(MechError {id: line!(),file: file!().to_string(),tokens: vec![],msg: format!("Schema mismatch: column {} not found in record",col_id),kind: MechErrorKind::None,});
      }
    }

    // Check that the data kinds match
    for (&col_id, (expected_kind, _)) in &self.data {
      if let Some((record_kind, _)) = record.data.get(&col_id) {
        if expected_kind != record_kind {
          return Err(MechError {id: line!(),file: file!().to_string(),tokens: vec![],msg: format!("Schema mismatch: column {} kind mismatch (expected: {:?}, found: {:?})",col_id, expected_kind, record_kind),kind: MechErrorKind::None,});
        }
      } else {
        return Err(MechError {id: line!(),file: file!().to_string(),tokens: vec![],msg: format!("Schema mismatch: column {} not found in record",col_id),kind: MechErrorKind::None,});
      }
    }

    Ok(true)
  }

  pub fn append_table(&mut self, other: &MechTable) -> MResult<()> {
    self.check_table_schema(other)?;
    for (&col_id, (_, other_matrix)) in &other.data {
      let (_, self_matrix) = self.data.get_mut(&col_id).ok_or(MechError {
        id: line!(),
        file: file!().to_string(),
        tokens: vec![],
        msg: format!("Column {} not found in destination table", col_id),
        kind: MechErrorKind::None,
      })?;

      self_matrix.append(other_matrix).map_err(|err| MechError {
        id: line!(),
        file: file!().to_string(),
        tokens: vec![],
        msg: "".to_string(),
        kind: MechErrorKind::None,
      })?;
    }
    self.rows += other.rows;
    Ok(())
  }

  pub fn append_record(&mut self, record: MechRecord) -> MResult<()> {
    // Validate schema (this includes column count, types, and optional name checks)
    self.check_record_schema(&record)?;

    // Append each value to the corresponding column in the matrix
    for (&col_id, value) in &record.data {
      if let Some((_kind, column_matrix)) = self.data.get_mut(&col_id) {
        let result = column_matrix.push(value.clone());
      } else {
        continue;
      }
    }

    // Increment row count
    self.rows += 1;

    Ok(())
  }

  pub fn get_record(&self, ix: usize) -> Option<MechRecord> {
    if ix > self.rows {
      return None;
    }

    let mut data: IndexMap<u64, Value> = IndexMap::new();
    data = self.data.iter().map(|(key, (kind, matrix))| {
      let value = matrix.index1d(ix);
      let name = self.col_names.get(key).unwrap();
      (hash_str(name), value.clone())
    }).collect();

    let mut kinds = Vec::with_capacity(self.cols);
    kinds = self.data.iter().map(|(_, (kind, _))| kind.clone()).collect();

    let mut field_names = self.col_names.clone();
   
    Some(MechRecord{cols: self.cols, kinds, data, field_names})
  }

  #[cfg(feature = "pretty_print")]
  pub fn to_html(&self) -> String {
    let mut html = String::new();

    // Start table
    html.push_str("<table class=\"mech-table\">");

    // Build thead
    html.push_str("<thead class=\"mech-table-header\"><tr>");
    for (key, (kind, _matrix)) in self.data.iter() {
        let col_name = self
            .col_names
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.to_string());

        let kind_str = format!(
            "<span class=\"mech-kind-annotation\">&lt;<span class=\"mech-kind\">{}</span>&gt;</span>",
            kind
        );

        html.push_str(&format!(
            "<th class=\"mech-table-field\">\
                <div class=\"mech-field\">\
                  <span class=\"mech-field-name\">{}</span>\
                  <span class=\"mech-field-kind\">{}</span>\
                </div>\
            </th>",
            col_name, kind_str
        ));
    }
    html.push_str("</tr></thead>");

    // Build tbody
    html.push_str("<tbody class=\"mech-table-body\">");
    for row_idx in 1..=self.rows {
        html.push_str("<tr class=\"mech-table-row\">");
        for (_key, (_kind, matrix)) in self.data.iter() {
            let value = matrix.index1d(row_idx);
            html.push_str(&format!(
                "<td class=\"mech-table-column\">{}</td>",
                value.to_html()
            ));
        }
        html.push_str("</tr>");
    }
    html.push_str("</tbody></table>");
    html
  }

  pub fn new(rows: usize, cols: usize, data: IndexMap<u64,(ValueKind,Matrix<Value>)>, col_names: HashMap<u64,String>) -> MechTable {
    MechTable{rows, cols, data, col_names}
  }

  pub fn kind(&self) -> ValueKind {
    let column_kinds: Vec<(String, ValueKind)> = self.data.iter()
      .filter_map(|(key, (kind, _))| {
        let col_name = self.col_names.get(key)?;
        Some((col_name.clone(), kind.clone()))
      })
      .collect();
    ValueKind::Table(column_kinds, self.rows)
  }
  
  pub fn size_of(&self) -> usize {
    self.data.iter().map(|(_,(_,v))| v.size_of()).sum()
  }

  pub fn rows(&self) -> usize {
    self.rows
  }

  pub fn cols(&self) -> usize {
    self.cols
  }

  pub fn get(&self, key: &u64) -> Option<&(ValueKind,Matrix<Value>)> {
    self.data.get(key)
  }

   pub fn shape(&self) -> Vec<usize> {
    vec![self.rows,self.cols]
  }
}

#[cfg(feature = "pretty_print")]
impl PrettyPrint for MechTable {
  fn pretty_print(&self) -> String {
    let mut builder = Builder::default();
    for (k,(knd,val)) in &self.data {
      let name = self.col_names.get(k).unwrap();
      let val_string: String = val.as_vec().iter()
        .map(|x| x.pretty_print())
        .collect::<Vec<String>>()
        .join("\n");
      let mut col_string = vec![format!("{}<{}>", name.to_string(), knd), val_string];
      builder.push_column(col_string);
    }
    let mut table = builder.build();
    table.with(Style::modern_rounded());
    format!("{table}")
  }
}

#[cfg(feature = "table")]
impl Hash for MechTable {
  fn hash<H: Hasher>(&self, state: &mut H) {
    for (k,(knd,val)) in self.data.iter() {
      k.hash(state);
      knd.hash(state);
      val.hash(state);
    }
  }
}