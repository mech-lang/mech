use super::{EnumVariantSchema, FloatWidth, IntegerWidth, Schema, SchemaBody, SchemaField};
use crate::dimension::{encode_dimension_parameters, encode_normalized_dimension};
use crate::{DimensionParameter, ExtentEvolution, SchemaKey, extent_evolution};
use sha2::{Digest, Sha256};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, vec::Vec};

struct CanonicalWriter {
    bytes: Vec<u8>,
}

impl CanonicalWriter {
    fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    fn write_u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn write_u16_le(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn write_u32_le(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn write_u64_le(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn write_bytes(&mut self, value: &[u8]) {
        self.bytes.extend_from_slice(value);
    }

    fn write_utf8(&mut self, value: &str) {
        self.write_u64_le(value.len() as u64);
        self.write_bytes(value.as_bytes());
    }

    fn write_node(&mut self, value: &[u8]) {
        self.write_u64_le(value.len() as u64);
        self.write_bytes(value);
    }

    fn finish(self) -> Box<[u8]> {
        self.bytes.into_boxed_slice()
    }
}

impl Schema {
    pub fn canonical_bytes(&self) -> Box<[u8]> {
        let mut writer = CanonicalWriter::new();
        writer.write_u8(0x01);
        writer.write_u32_le(self.dimension_parameters.len() as u32);
        encode_dimension_parameters(&self.dimension_parameters, &mut writer.bytes);
        let body = encode_schema_body(&self.body);
        writer.write_node(&body);
        writer.finish()
    }

    pub fn key(&self) -> SchemaKey {
        let mut hash = Sha256::new();
        hash.update(b"mech-schema-v1\0");
        hash.update(self.canonical_bytes());
        SchemaKey::from_bytes(hash.finalize().into())
    }

    pub fn is_keyable(&self) -> bool {
        super::validation::is_body_keyable(&self.body)
    }

    pub fn extent_evolution(&self) -> ExtentEvolution {
        extent_evolution(&self.dimension_parameters)
    }

    pub fn dimension_parameters(&self) -> &[DimensionParameter] {
        &self.dimension_parameters
    }

    pub const fn body(&self) -> &SchemaBody {
        &self.body
    }
}

fn encode_schema_body(body: &SchemaBody) -> Box<[u8]> {
    let mut writer = CanonicalWriter::new();
    match body {
        SchemaBody::Dynamic => writer.write_u8(0x14),
        SchemaBody::Bool => writer.write_u8(0x01),
        SchemaBody::UnsignedInteger(width) => {
            writer.write_u8(0x02);
            writer.write_u16_le(integer_width(*width));
        }
        SchemaBody::SignedInteger(width) => {
            writer.write_u8(0x03);
            writer.write_u16_le(integer_width(*width));
        }
        SchemaBody::FloatingPoint(width) => {
            writer.write_u8(0x04);
            writer.write_u16_le(float_width(*width));
        }
        SchemaBody::Complex(width) => {
            writer.write_u8(0x05);
            writer.write_u16_le(float_width(*width));
        }
        SchemaBody::Rational64 => {
            writer.write_u8(0x06);
            writer.write_u16_le(64);
            writer.write_u16_le(64);
        }
        SchemaBody::String => writer.write_u8(0x07),
        SchemaBody::Id => writer.write_u8(0x08),
        SchemaBody::Index => writer.write_u8(0x09),
        SchemaBody::Atom(key) => {
            writer.write_u8(0x0a);
            writer.write_bytes(key.as_bytes());
        }
        SchemaBody::Enum { key, variants } => {
            writer.write_u8(0x0b);
            writer.write_bytes(key.as_bytes());
            encode_variants(&mut writer, variants);
        }
        SchemaBody::Option(element) => {
            writer.write_u8(0x0c);
            writer.write_node(&encode_schema_body(element));
        }
        SchemaBody::Tuple(elements) => {
            writer.write_u8(0x0d);
            writer.write_u32_le(elements.len() as u32);
            for element in elements {
                writer.write_node(&encode_schema_body(element));
            }
        }
        SchemaBody::Record(fields) => {
            writer.write_u8(0x0e);
            encode_fields(&mut writer, fields);
        }
        SchemaBody::Matrix {
            element,
            dimensions,
        } => {
            writer.write_u8(0x0f);
            writer.write_node(&encode_schema_body(element));
            writer.write_u32_le(dimensions.len() as u32);
            for dimension in dimensions {
                writer.write_node(&encode_normalized_dimension(dimension));
            }
        }
        SchemaBody::Table { columns, rows } => {
            writer.write_u8(0x10);
            encode_fields(&mut writer, columns);
            writer.write_node(&encode_normalized_dimension(rows));
        }
        SchemaBody::Set {
            element,
            cardinality,
        } => {
            writer.write_u8(0x11);
            writer.write_node(&encode_schema_body(element));
            writer.write_node(&encode_normalized_dimension(cardinality));
        }
        SchemaBody::Map {
            key,
            value,
            cardinality,
        } => {
            writer.write_u8(0x12);
            writer.write_node(&encode_schema_body(key));
            writer.write_node(&encode_schema_body(value));
            writer.write_node(&encode_normalized_dimension(cardinality));
        }
        SchemaBody::ReifiedType => writer.write_u8(0x13),
    }
    writer.finish()
}

fn encode_variants(writer: &mut CanonicalWriter, variants: &[EnumVariantSchema]) {
    writer.write_u32_le(variants.len() as u32);
    for variant in variants {
        writer.write_utf8(&variant.name);
        match &variant.payload {
            None => writer.write_u8(0),
            Some(payload) => {
                writer.write_u8(1);
                writer.write_node(&encode_schema_body(payload));
            }
        }
    }
}

fn encode_fields(writer: &mut CanonicalWriter, fields: &[SchemaField]) {
    writer.write_u32_le(fields.len() as u32);
    for field in fields {
        writer.write_utf8(&field.name);
        writer.write_node(&encode_schema_body(&field.schema));
    }
}

const fn integer_width(width: IntegerWidth) -> u16 {
    width as u16
}

const fn float_width(width: FloatWidth) -> u16 {
    width as u16
}
