//! Generic tree traversal
//!
//! This module defines functionality that allows untyped access to font table
//! data. This is used as the basis for things like debug printing.

use std::{fmt::Debug, ops::Deref};

use font_types::{
    BigEndian, F2Dot14, FWord, Fixed, GlyphId, LongDateTime, MajorMinor, Nullable, Offset16,
    Offset24, Offset32, Scalar, Tag, UfWord, Uint24, Version16Dot16,
};

use crate::{
    array::ComputedArray,
    layout::gpos::ValueRecord,
    read::{ComputeSize, ReadArgs},
    FontData, FontReadWithArgs, ReadError,
};

/// Types of fields in font tables.
///
/// Fields can either be scalars, offsets to tables, or arrays.
pub enum FieldType<'a> {
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    U24(Uint24),
    Tag(Tag),
    FWord(FWord),
    UfWord(UfWord),
    MajorMinor(MajorMinor),
    Version16Dot16(Version16Dot16),
    F2Dot14(F2Dot14),
    Fixed(Fixed),
    LongDateTime(LongDateTime),
    GlyphId(GlyphId),
    BareOffset(OffsetType),
    ResolvedOffset(ResolvedOffset<'a>),
    Record(RecordResolver<'a>),
    ValueRecord(ValueRecord),
    Array(Box<dyn SomeArray<'a> + 'a>),
    // used for fields in other versions of a table
    None,
}

#[derive(Clone, Copy)]
pub enum OffsetType {
    None,
    Offset16(u16),
    Offset24(Uint24),
    Offset32(u32),
}

impl OffsetType {
    pub fn to_u32(self) -> u32 {
        match self {
            Self::None => 0,
            Self::Offset16(val) => val.into(),
            Self::Offset24(val) => val.into(),
            Self::Offset32(val) => val,
        }
    }
}

pub struct ResolvedOffset<'a> {
    pub offset: OffsetType,
    pub target: Result<Box<dyn SomeTable<'a> + 'a>, ReadError>,
}

pub struct ArrayOfOffsets<'a, O> {
    type_name: &'static str,
    offsets: &'a [O],
    resolver: Box<dyn Fn(&O) -> FieldType<'a> + 'a>,
}

impl<'a, O> SomeArray<'a> for ArrayOfOffsets<'a, O> {
    fn type_name(&self) -> &str {
        self.type_name
    }

    fn len(&self) -> usize {
        self.offsets.len()
    }

    fn get(&self, idx: usize) -> Option<FieldType<'a>> {
        let off = self.offsets.get(idx)?;
        let target = (self.resolver)(off);
        Some(target)
    }
}

impl<'a> FieldType<'a> {
    /// makes a field, handling the case where this array may not be present in
    /// all versions
    pub fn array_of_records<T>(
        type_name: &'static str,
        records: impl Into<Option<&'a [T]>>,
        data: FontData<'a>,
    ) -> FieldType<'a>
    where
        T: Clone + SomeRecord<'a> + 'a,
    {
        match records.into() {
            None => FieldType::None,
            Some(records) => ArrayOfRecords {
                type_name,
                data,
                records,
            }
            .into(),
        }
    }

    // Convenience method for handling computed arrays
    pub fn computed_array<T>(
        type_name: &'static str,
        array: impl Into<Option<ComputedArray<'a, T>>>,
        data: FontData<'a>,
    ) -> FieldType<'a>
    where
        T: FontReadWithArgs<'a> + ComputeSize + SomeRecord<'a> + 'a,
        T::Args: Copy + 'static,
    {
        match array.into() {
            None => FieldType::None,
            Some(array) => ComputedArrayOfRecords {
                type_name,
                data,
                array,
            }
            .into(),
        }
    }

    pub fn offset_array<O>(
        type_name: &'static str,
        offsets: &'a [O],
        resolver: impl Fn(&O) -> FieldType<'a> + 'a,
    ) -> Self
where {
        FieldType::Array(Box::new(ArrayOfOffsets {
            type_name,
            offsets,
            resolver: Box::new(resolver),
        }))
    }

    //FIXME: I bet this is generating a *lot* of code
    pub fn offset<T: SomeTable<'a> + 'a>(
        offset: impl Into<OffsetType>,
        result: impl Into<Option<Result<T, ReadError>>>,
    ) -> Self {
        let offset = offset.into();
        match result.into() {
            Some(target) => FieldType::ResolvedOffset(ResolvedOffset {
                offset,
                target: target.map(|x| Box::new(x) as Box<dyn SomeTable>),
            }),
            None => FieldType::BareOffset(offset),
        }
    }

    pub fn unknown_offset(offset: impl Into<OffsetType>) -> Self {
        Self::BareOffset(offset.into())
    }
}

/// A generic field in a font table
pub struct Field<'a> {
    pub name: &'static str,
    pub typ: FieldType<'a>,
}

/// A generic table type.
///
/// This is intended to be used as a trait object.
pub trait SomeTable<'a> {
    /// The name of this table
    fn type_name(&self) -> &str;
    /// Access this table's fields, in declaration order.
    fn get_field(&self, idx: usize) -> Option<Field<'a>>;
}

impl<'a> dyn SomeTable<'a> + 'a {
    pub fn iter(&self) -> impl Iterator<Item = Field<'a>> + '_ {
        FieldIter {
            table: self,
            idx: 0,
        }
    }
}

impl<'a> SomeTable<'a> for Box<dyn SomeTable<'a> + 'a> {
    fn type_name(&self) -> &str {
        self.deref().type_name()
    }

    fn get_field(&self, idx: usize) -> Option<Field<'a>> {
        self.deref().get_field(idx)
    }
}

/// A generic trait for records, which need to be passed in data
/// in order to fully resolve themselves.
pub trait SomeRecord<'a> {
    fn traverse(self, data: FontData<'a>) -> RecordResolver<'a>;
}

/// A struct created from a record and the data it needs to resolve any
/// contained offsets.
pub struct RecordResolver<'a> {
    pub(crate) name: &'static str,
    pub(crate) get_field: Box<dyn Fn(usize, FontData<'a>) -> Option<Field<'a>> + 'a>,
    pub(crate) data: FontData<'a>,
}

pub trait SomeArray<'a> {
    fn type_name(&self) -> &str;
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn get(&self, idx: usize) -> Option<FieldType<'a>>;
}

impl<'a> dyn SomeArray<'a> + 'a {
    pub fn iter(&self) -> impl Iterator<Item = FieldType<'a>> + '_ {
        ArrayIter {
            array: self,
            idx: 0,
        }
    }
}

impl<'a, T: Scalar + Into<FieldType<'a>>> SomeArray<'a> for &'a [BigEndian<T>]
where
    BigEndian<T>: Copy, // i don't know why i need this??
{
    fn len(&self) -> usize {
        (*self).len()
    }

    fn get(&self, idx: usize) -> Option<FieldType<'a>> {
        (*self).get(idx).map(|val| val.get().into())
    }

    fn type_name(&self) -> &str {
        let full_name = std::any::type_name::<T>();
        full_name.split("::").last().unwrap_or(full_name)
    }
}

impl<'a> SomeArray<'a> for &'a [u8] {
    fn type_name(&self) -> &str {
        "u8"
    }

    fn len(&self) -> usize {
        (*self).len()
    }

    fn get(&self, idx: usize) -> Option<FieldType<'a>> {
        (*self).get(idx).copied().map(Into::into)
    }
}

impl<'a> SomeArray<'a> for Box<dyn SomeArray<'a> + 'a> {
    fn type_name(&self) -> &str {
        self.deref().type_name()
    }

    fn len(&self) -> usize {
        self.deref().len()
    }

    fn get(&self, idx: usize) -> Option<FieldType<'a>> {
        self.deref().get(idx)
    }
}

// only used as Box<dyn SomeArray<'a>>
struct ComputedArrayOfRecords<'a, T: ReadArgs> {
    pub(crate) type_name: &'static str,
    pub(crate) data: FontData<'a>,
    pub(crate) array: ComputedArray<'a, T>,
}

impl<'a, T> SomeArray<'a> for ComputedArrayOfRecords<'a, T>
where
    T: FontReadWithArgs<'a> + ComputeSize + SomeRecord<'a> + 'a,
    T::Args: Copy + 'static,
    Self: 'a,
{
    fn len(&self) -> usize {
        self.array.len()
    }

    fn get(&self, idx: usize) -> Option<FieldType<'a>> {
        self.array
            .get(idx)
            .ok()
            .map(|record| record.traverse(self.data).into())
    }

    fn type_name(&self) -> &str {
        self.type_name
    }
}

// only used as Box<dyn SomeArray<'a>>
struct ArrayOfRecords<'a, T> {
    pub(crate) type_name: &'static str,
    pub(crate) data: FontData<'a>,
    pub(crate) records: &'a [T],
}

impl<'a, T: SomeRecord<'a> + Clone> SomeArray<'a> for ArrayOfRecords<'a, T> {
    fn type_name(&self) -> &str {
        self.type_name
    }

    fn len(&self) -> usize {
        self.records.len()
    }

    fn get(&self, idx: usize) -> Option<FieldType<'a>> {
        self.records
            .get(idx)
            .map(|record| record.clone().traverse(self.data).into())
    }
}

struct FieldIter<'a, 'b> {
    table: &'b dyn SomeTable<'a>,
    idx: usize,
}

impl<'a, 'b> Iterator for FieldIter<'a, 'b> {
    type Item = Field<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let this = self.idx;
        self.idx += 1;
        self.table.get_field(this)
    }
}

struct ArrayIter<'a, 'b> {
    array: &'b dyn SomeArray<'a>,
    idx: usize,
}

impl<'a, 'b> Iterator for ArrayIter<'a, 'b> {
    type Item = FieldType<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let this = self.idx;
        self.idx += 1;
        self.array.get(this)
    }
}

impl<'a> Field<'a> {
    pub fn new(name: &'static str, typ: impl Into<FieldType<'a>>) -> Self {
        Field {
            name,
            typ: typ.into(),
        }
    }
}

/// A wrapper type that implements `Debug` for any table.
struct DebugPrintTable<'a, 'b>(pub &'b (dyn SomeTable<'a> + 'a));

/// A wrapper type that implements `Debug` for any array.
struct DebugPrintArray<'a, 'b>(pub &'b (dyn SomeArray<'a> + 'a));

impl<'a> Debug for FieldType<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::I8(arg0) => arg0.fmt(f),
            Self::U8(arg0) => arg0.fmt(f),
            Self::I16(arg0) => arg0.fmt(f),
            Self::U16(arg0) => arg0.fmt(f),
            Self::I32(arg0) => arg0.fmt(f),
            Self::U32(arg0) => arg0.fmt(f),
            Self::U24(arg0) => arg0.fmt(f),
            Self::Tag(arg0) => arg0.fmt(f),
            Self::FWord(arg0) => arg0.to_i16().fmt(f),
            Self::UfWord(arg0) => arg0.to_u16().fmt(f),
            Self::MajorMinor(arg0) => write!(f, "{}.{}", arg0.major, arg0.minor),
            Self::Version16Dot16(arg0) => arg0.fmt(f),
            Self::F2Dot14(arg0) => arg0.fmt(f),
            Self::Fixed(arg0) => arg0.fmt(f),
            Self::LongDateTime(arg0) => arg0.as_secs().fmt(f),
            Self::GlyphId(arg0) => {
                write!(f, "g")?;
                arg0.to_u16().fmt(f)
            }
            Self::BareOffset(arg0) => write!(f, "0x{:04X}", arg0.to_u32()),
            Self::None => write!(f, "None"),
            Self::ResolvedOffset(ResolvedOffset {
                target: Ok(arg0), ..
            }) => arg0.fmt(f),
            Self::ResolvedOffset(arg0) => arg0.target.fmt(f),
            Self::Record(arg0) => (arg0 as &(dyn SomeTable<'a> + 'a)).fmt(f),
            Self::ValueRecord(arg0) if arg0.get_field(0).is_none() => write!(f, "NullValueRecord"),
            Self::ValueRecord(arg0) => (arg0 as &(dyn SomeTable<'a> + 'a)).fmt(f),
            Self::Array(arg0) => arg0.fmt(f),
        }
    }
}

impl<'a, 'b> std::fmt::Debug for DebugPrintTable<'a, 'b> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut debug_struct = f.debug_struct(self.0.type_name());
        for field in self.0.iter() {
            debug_struct.field(field.name, &field.typ);
        }
        debug_struct.finish()
    }
}

impl<'a> Debug for dyn SomeTable<'a> + 'a {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        DebugPrintTable(self).fmt(f)
    }
}

impl<'a, 'b> std::fmt::Debug for DebugPrintArray<'a, 'b> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut debug_list = f.debug_list();
        for item in self.0.iter() {
            debug_list.entry(&item);
        }
        debug_list.finish()
    }
}

impl<'a> Debug for dyn SomeArray<'a> + 'a {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        DebugPrintArray(self).fmt(f)
    }
}

// used to give us an auto-impl of Debug
impl<'a> SomeTable<'a> for RecordResolver<'a> {
    fn type_name(&self) -> &str {
        self.name
    }

    fn get_field(&self, idx: usize) -> Option<Field<'a>> {
        (self.get_field)(idx, self.data)
    }
}

impl<'a> From<u8> for FieldType<'a> {
    fn from(src: u8) -> FieldType<'a> {
        FieldType::U8(src)
    }
}

impl<'a> From<i8> for FieldType<'a> {
    fn from(src: i8) -> FieldType<'a> {
        FieldType::I8(src)
    }
}

impl<'a> From<u16> for FieldType<'a> {
    fn from(src: u16) -> FieldType<'a> {
        FieldType::U16(src)
    }
}

impl<'a> From<i16> for FieldType<'a> {
    fn from(src: i16) -> FieldType<'a> {
        FieldType::I16(src)
    }
}

impl<'a> From<u32> for FieldType<'a> {
    fn from(src: u32) -> FieldType<'a> {
        FieldType::U32(src)
    }
}

impl<'a> From<i32> for FieldType<'a> {
    fn from(src: i32) -> FieldType<'a> {
        FieldType::I32(src)
    }
}

impl<'a> From<Uint24> for FieldType<'a> {
    fn from(src: Uint24) -> FieldType<'a> {
        FieldType::U24(src)
    }
}

impl<'a> From<Tag> for FieldType<'a> {
    fn from(src: Tag) -> FieldType<'a> {
        FieldType::Tag(src)
    }
}

impl<'a> From<FWord> for FieldType<'a> {
    fn from(src: FWord) -> FieldType<'a> {
        FieldType::FWord(src)
    }
}

impl<'a> From<UfWord> for FieldType<'a> {
    fn from(src: UfWord) -> FieldType<'a> {
        FieldType::UfWord(src)
    }
}

impl<'a> From<Fixed> for FieldType<'a> {
    fn from(src: Fixed) -> FieldType<'a> {
        FieldType::Fixed(src)
    }
}

impl<'a> From<F2Dot14> for FieldType<'a> {
    fn from(src: F2Dot14) -> FieldType<'a> {
        FieldType::F2Dot14(src)
    }
}

impl<'a> From<LongDateTime> for FieldType<'a> {
    fn from(src: LongDateTime) -> FieldType<'a> {
        FieldType::LongDateTime(src)
    }
}

impl<'a> From<MajorMinor> for FieldType<'a> {
    fn from(src: MajorMinor) -> FieldType<'a> {
        FieldType::MajorMinor(src)
    }
}

impl<'a> From<Version16Dot16> for FieldType<'a> {
    fn from(src: Version16Dot16) -> FieldType<'a> {
        FieldType::Version16Dot16(src)
    }
}

impl<'a> From<GlyphId> for FieldType<'a> {
    fn from(src: GlyphId) -> FieldType<'a> {
        FieldType::GlyphId(src)
    }
}

impl<'a, T: Into<FieldType<'a>>> From<Option<T>> for FieldType<'a> {
    fn from(src: Option<T>) -> Self {
        match src {
            Some(t) => t.into(),
            None => FieldType::None,
        }
    }
}

impl<'a> From<ValueRecord> for FieldType<'a> {
    fn from(src: ValueRecord) -> Self {
        Self::ValueRecord(src)
    }
}

impl<'a> From<RecordResolver<'a>> for FieldType<'a> {
    fn from(src: RecordResolver<'a>) -> Self {
        FieldType::Record(src)
    }
}

impl<'a, T: SomeArray<'a> + 'a> From<T> for FieldType<'a> {
    fn from(src: T) -> Self {
        FieldType::Array(Box::new(src))
    }
}

impl From<Offset16> for OffsetType {
    fn from(src: Offset16) -> OffsetType {
        OffsetType::Offset16(src.to_u32() as u16)
    }
}

impl From<Offset24> for OffsetType {
    fn from(src: Offset24) -> OffsetType {
        OffsetType::Offset24(Uint24::new(src.to_u32()))
    }
}

impl From<Offset32> for OffsetType {
    fn from(src: Offset32) -> OffsetType {
        OffsetType::Offset32(src.to_u32())
    }
}

impl<'a> From<Offset16> for FieldType<'a> {
    fn from(src: Offset16) -> FieldType<'a> {
        FieldType::BareOffset(src.into())
    }
}

impl<'a> From<Offset24> for FieldType<'a> {
    fn from(src: Offset24) -> FieldType<'a> {
        FieldType::BareOffset(src.into())
    }
}

impl<'a> From<Offset32> for FieldType<'a> {
    fn from(src: Offset32) -> FieldType<'a> {
        FieldType::BareOffset(src.into())
    }
}

impl<T: Into<OffsetType> + Clone> From<Nullable<T>> for OffsetType {
    fn from(src: Nullable<T>) -> Self {
        src.offset().clone().into()
    }
}

impl<T: Into<OffsetType> + Clone> From<Option<Nullable<T>>> for OffsetType {
    fn from(src: Option<Nullable<T>>) -> Self {
        match src {
            None => OffsetType::None,
            Some(off) => off.into(),
        }
    }
}
