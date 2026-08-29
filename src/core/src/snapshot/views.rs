use super::{
    CanonicalKeyValue, MapEntryValue, MapValue, MatrixValue, RecordValue, SequenceView, SetValue,
    TableValue, Value, ValueData,
};

#[derive(Clone, Copy, Debug)]
pub struct EnumView<'a> {
    ordinal: u32,
    payload: Option<&'a ValueData>,
}

impl<'a> EnumView<'a> {
    pub const fn ordinal(self) -> u32 {
        self.ordinal
    }

    pub const fn payload(self) -> Option<&'a ValueData> {
        self.payload
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TupleView<'a>(&'a [ValueData]);

impl<'a> TupleView<'a> {
    pub const fn elements(self) -> &'a [ValueData] {
        self.0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RecordView<'a>(&'a RecordValue);

impl<'a> RecordView<'a> {
    pub fn fields(self) -> &'a [ValueData] {
        self.0.fields()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MatrixView<'a>(&'a MatrixValue);

impl<'a> MatrixView<'a> {
    pub fn elements(self) -> SequenceView<'a> {
        self.0.elements()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TableView<'a>(&'a TableValue);

impl<'a> TableView<'a> {
    pub fn len(self) -> usize {
        self.0.len()
    }

    pub fn is_empty(self) -> bool {
        self.0.is_empty()
    }

    pub fn column(self, index: usize) -> Option<SequenceView<'a>> {
        self.0.column(index)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SetView<'a>(&'a SetValue);

impl<'a> SetView<'a> {
    pub fn elements(self) -> &'a [CanonicalKeyValue] {
        self.0.elements()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MapView<'a>(&'a MapValue);

impl<'a> MapView<'a> {
    pub fn entries(self) -> &'a [MapEntryValue] {
        self.0.entries()
    }
}

impl Value {
    pub fn enum_view(&self) -> Option<EnumView<'_>> {
        let ValueData::Enum(value) = self.data() else {
            return None;
        };
        Some(EnumView {
            ordinal: value.ordinal(),
            payload: value.payload(),
        })
    }

    pub fn option_view(&self) -> Option<Option<&ValueData>> {
        let ValueData::Option(value) = self.data() else {
            return None;
        };
        Some(value.as_deref())
    }

    pub fn tuple_view(&self) -> Option<TupleView<'_>> {
        let ValueData::Tuple(value) = self.data() else {
            return None;
        };
        Some(TupleView(value))
    }

    pub fn record_view(&self) -> Option<RecordView<'_>> {
        let ValueData::Record(value) = self.data() else {
            return None;
        };
        Some(RecordView(value))
    }

    pub fn matrix_view(&self) -> Option<MatrixView<'_>> {
        let ValueData::Matrix(value) = self.data() else {
            return None;
        };
        Some(MatrixView(value))
    }

    pub fn table_view(&self) -> Option<TableView<'_>> {
        let ValueData::Table(value) = self.data() else {
            return None;
        };
        Some(TableView(value))
    }

    pub fn set_view(&self) -> Option<SetView<'_>> {
        let ValueData::Set(value) = self.data() else {
            return None;
        };
        Some(SetView(value))
    }

    pub fn map_view(&self) -> Option<MapView<'_>> {
        let ValueData::Map(value) = self.data() else {
            return None;
        };
        Some(MapView(value))
    }
}
