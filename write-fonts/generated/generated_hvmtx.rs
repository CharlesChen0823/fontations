// THIS FILE IS AUTOGENERATED.
// Any changes to this file will be overwritten.
// For more information about how codegen works, see font-codegen/README.md

#[allow(unused_imports)]
use crate::codegen_prelude::*;

/// The [hmtx (Horizontal Metrics)](https://docs.microsoft.com/en-us/typography/opentype/spec/hmtx) table
/// The [vmtx (Vertical Metrics)](https://docs.microsoft.com/en-us/typography/opentype/spec/vmtx) table
#[derive(Clone, Debug, Default)]
pub struct HVmtx {
    /// Paired advance width/height and left/top side bearing values for each
    /// glyph. Records are indexed by glyph ID.
    pub long_metrics: Vec<LongMetric>,
    /// Leading (left/top) side bearings for glyph IDs greater than or equal to
    /// numberOfLongMetrics.
    pub bearings: Vec<i16>,
}

impl HVmtx {
    /// Construct a new `HVmtx`
    pub fn new(long_metrics: Vec<LongMetric>, bearings: Vec<i16>) -> Self {
        Self {
            long_metrics: long_metrics.into_iter().map(Into::into).collect(),
            bearings: bearings.into_iter().map(Into::into).collect(),
        }
    }
}

impl FontWrite for HVmtx {
    fn write_into(&self, writer: &mut TableWriter) {
        self.long_metrics.write_into(writer);
        self.bearings.write_into(writer);
    }
}

impl Validate for HVmtx {
    fn validate_impl(&self, ctx: &mut ValidationCtx) {
        ctx.in_table("HVmtx", |ctx| {
            ctx.in_field("long_metrics", |ctx| {
                if self.long_metrics.len() > (u16::MAX as usize) {
                    ctx.report("array excedes max length");
                }
                self.long_metrics.validate_impl(ctx);
            });
        })
    }
}

impl<'a> FromObjRef<read_fonts::tables::hvmtx::HVmtx<'a>> for HVmtx {
    fn from_obj_ref(obj: &read_fonts::tables::hvmtx::HVmtx<'a>, _: FontData) -> Self {
        let offset_data = obj.offset_data();
        HVmtx {
            long_metrics: obj.long_metrics().to_owned_obj(offset_data),
            bearings: obj.bearings().to_owned_obj(offset_data),
        }
    }
}

impl<'a> FromTableRef<read_fonts::tables::hvmtx::HVmtx<'a>> for HVmtx {}

#[derive(Clone, Debug, Default)]
pub struct LongMetric {
    /// Advance width/height, in font design units.
    pub advance: u16,
    /// Glyph leading (left/top) side bearing, in font design units.
    pub side_bearing: i16,
}

impl LongMetric {
    /// Construct a new `LongMetric`
    pub fn new(advance: u16, side_bearing: i16) -> Self {
        Self {
            advance,
            side_bearing,
        }
    }
}

impl FontWrite for LongMetric {
    fn write_into(&self, writer: &mut TableWriter) {
        self.advance.write_into(writer);
        self.side_bearing.write_into(writer);
    }
}

impl Validate for LongMetric {
    fn validate_impl(&self, _ctx: &mut ValidationCtx) {}
}

impl FromObjRef<read_fonts::tables::hvmtx::LongMetric> for LongMetric {
    fn from_obj_ref(obj: &read_fonts::tables::hvmtx::LongMetric, _: FontData) -> Self {
        LongMetric {
            advance: obj.advance(),
            side_bearing: obj.side_bearing(),
        }
    }
}
