// THIS FILE IS AUTOGENERATED.
// Any changes to this file will be overwritten.
// For more information about how codegen works, see font-codegen/README.md

#[allow(unused_imports)]
use crate::codegen_prelude::*;

pub use read_fonts::tables::os2::SelectionFlags;

impl FontWrite for SelectionFlags {
    fn write_into(&self, writer: &mut TableWriter) {
        writer.write_slice(&self.bits().to_be_bytes())
    }
}

/// [`OS/2`](https://docs.microsoft.com/en-us/typography/opentype/spec/os2)
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Os2 {
    /// [Average weighted escapement](https://learn.microsoft.com/en-us/typography/opentype/spec/os2#xavgcharwidth).
    ///
    /// The Average Character Width parameter specifies the arithmetic average
    /// of the escapement (width) of all non-zero width glyphs in the font.
    pub x_avg_char_width: i16,
    /// [Weight class](https://learn.microsoft.com/en-us/typography/opentype/spec/os2#usweightclass).
    ///
    /// Indicates the visual weight (degree of blackness or thickness of
    /// strokes) of the characters in the font. Values from 1 to 1000 are valid.
    pub us_weight_class: u16,
    /// [Width class](https://learn.microsoft.com/en-us/typography/opentype/spec/os2#uswidthclass).
    ///
    /// Indicates a relative change from the normal aspect ratio (width to height
    /// ratio) as specified by a font designer for the glyphs in a font.
    pub us_width_class: u16,
    /// [Type flags](https://learn.microsoft.com/en-us/typography/opentype/spec/os2#fstype).
    ///
    /// Indicates font embedding licensing rights for the font.
    pub fs_type: u16,
    /// The recommended horizontal size in font design units for subscripts for
    /// this font.
    pub y_subscript_x_size: i16,
    /// The recommended vertical size in font design units for subscripts for
    /// this font.
    pub y_subscript_y_size: i16,
    /// The recommended horizontal offset in font design units for subscripts
    /// for this font.
    pub y_subscript_x_offset: i16,
    /// The recommended vertical offset in font design units for subscripts
    /// for this font.
    pub y_subscript_y_offset: i16,
    /// The recommended horizontal size in font design units for superscripts
    /// for this font.
    pub y_superscript_x_size: i16,
    /// The recommended vertical size in font design units for superscripts
    /// for this font.
    pub y_superscript_y_size: i16,
    /// The recommended horizontal offset in font design units for superscripts
    /// for this font.
    pub y_superscript_x_offset: i16,
    /// The recommended vertical offset in font design units for superscripts
    /// for this font.
    pub y_superscript_y_offset: i16,
    /// Thickness of the strikeout stroke in font design units.
    pub y_strikeout_size: i16,
    /// The position of the top of the strikeout stroke relative to the
    /// baseline in font design units.
    pub y_strikeout_position: i16,
    /// [Font-family class and subclass](https://learn.microsoft.com/en-us/typography/opentype/spec/os2#sfamilyclass).
    /// This parameter is a classification of font-family design.
    pub s_family_class: i16,
    /// [PANOSE classification number](https://learn.microsoft.com/en-us/typography/opentype/spec/os2#panose).
    ///
    /// Additional specifications are required for PANOSE to classify non-Latin
    /// character sets.
    pub panose_10: [u8; 10],
    /// [Unicode Character Range](https://learn.microsoft.com/en-us/typography/opentype/spec/os2#ulunicoderange1-bits-031ulunicoderange2-bits-3263ulunicoderange3-bits-6495ulunicoderange4-bits-96127).
    ///
    /// Unicode Character Range (bits 0-31).
    pub ul_unicode_range_1: u32,
    /// Unicode Character Range (bits 32-63).
    pub ul_unicode_range_2: u32,
    /// Unicode Character Range (bits 64-95).
    pub ul_unicode_range_3: u32,
    /// Unicode Character Range (bits 96-127).
    pub ul_unicode_range_4: u32,
    /// [Font Vendor Identification](https://learn.microsoft.com/en-us/typography/opentype/spec/os2#achvendid).
    ///
    /// The four-character identifier for the vendor of the given type face.
    pub ach_vend_id: Tag,
    /// [Font selection flags](https://learn.microsoft.com/en-us/typography/opentype/spec/os2#fsselection).
    ///
    /// Contains information concerning the nature of the font patterns.
    pub fs_selection: SelectionFlags,
    /// The minimum Unicode index (character code) in this font.
    pub us_first_char_index: u16,
    /// The maximum Unicode index (character code) in this font.
    pub us_last_char_index: u16,
    /// The typographic ascender for this font.
    pub s_typo_ascender: i16,
    /// The typographic descender for this font.
    pub s_typo_descender: i16,
    /// The typographic line gap for this font.
    pub s_typo_line_gap: i16,
    /// The “Windows ascender” metric.
    ///
    /// This should be used to specify the height above the baseline for a
    /// clipping region.
    pub us_win_ascent: u16,
    /// The “Windows descender” metric.
    ///
    /// This should be used to specify the vertical extent below the baseline
    /// for a clipping region.
    pub us_win_descent: u16,
    /// Code page character range bits 0-31.
    pub ul_code_page_range_1: Option<u32>,
    /// Code page character range bits 32-63.
    pub ul_code_page_range_2: Option<u32>,
    /// This metric specifies the distance between the baseline and the
    /// approximate height of non-ascending lowercase letters measured in
    /// FUnits.
    pub sx_height: Option<i16>,
    /// This metric specifies the distance between the baseline and the
    /// approximate height of uppercase letters measured in FUnits.
    pub s_cap_height: Option<i16>,
    /// This is the Unicode code point, in UTF-16 encoding, of a character that
    /// can be used for a default glyph.
    pub us_default_char: Option<u16>,
    /// his is the Unicode code point, in UTF-16 encoding, of a character that
    /// can be used as a default break character.
    pub us_break_char: Option<u16>,
    /// This field is used for fonts with multiple optical styles.
    pub us_max_context: Option<u16>,
    /// This field is used for fonts with multiple optical styles.
    pub us_lower_optical_point_size: Option<u16>,
    /// This field is used for fonts with multiple optical styles.
    pub us_upper_optical_point_size: Option<u16>,
}

impl Default for Os2 {
    fn default() -> Self {
        Self {
            x_avg_char_width: Default::default(),
            us_weight_class: 400,
            us_width_class: 5,
            fs_type: Default::default(),
            y_subscript_x_size: Default::default(),
            y_subscript_y_size: Default::default(),
            y_subscript_x_offset: Default::default(),
            y_subscript_y_offset: Default::default(),
            y_superscript_x_size: Default::default(),
            y_superscript_y_size: Default::default(),
            y_superscript_x_offset: Default::default(),
            y_superscript_y_offset: Default::default(),
            y_strikeout_size: Default::default(),
            y_strikeout_position: Default::default(),
            s_family_class: Default::default(),
            panose_10: Default::default(),
            ul_unicode_range_1: Default::default(),
            ul_unicode_range_2: Default::default(),
            ul_unicode_range_3: Default::default(),
            ul_unicode_range_4: Default::default(),
            ach_vend_id: Default::default(),
            fs_selection: Default::default(),
            us_first_char_index: Default::default(),
            us_last_char_index: Default::default(),
            s_typo_ascender: Default::default(),
            s_typo_descender: Default::default(),
            s_typo_line_gap: Default::default(),
            us_win_ascent: Default::default(),
            us_win_descent: Default::default(),
            ul_code_page_range_1: Default::default(),
            ul_code_page_range_2: Default::default(),
            sx_height: Default::default(),
            s_cap_height: Default::default(),
            us_default_char: Default::default(),
            us_break_char: Default::default(),
            us_max_context: Default::default(),
            us_lower_optical_point_size: Default::default(),
            us_upper_optical_point_size: Default::default(),
        }
    }
}

impl FontWrite for Os2 {
    #[allow(clippy::unnecessary_cast)]
    fn write_into(&self, writer: &mut TableWriter) {
        let version = self.compute_version() as u16;
        version.write_into(writer);
        self.x_avg_char_width.write_into(writer);
        self.us_weight_class.write_into(writer);
        self.us_width_class.write_into(writer);
        self.fs_type.write_into(writer);
        self.y_subscript_x_size.write_into(writer);
        self.y_subscript_y_size.write_into(writer);
        self.y_subscript_x_offset.write_into(writer);
        self.y_subscript_y_offset.write_into(writer);
        self.y_superscript_x_size.write_into(writer);
        self.y_superscript_y_size.write_into(writer);
        self.y_superscript_x_offset.write_into(writer);
        self.y_superscript_y_offset.write_into(writer);
        self.y_strikeout_size.write_into(writer);
        self.y_strikeout_position.write_into(writer);
        self.s_family_class.write_into(writer);
        self.panose_10.write_into(writer);
        self.ul_unicode_range_1.write_into(writer);
        self.ul_unicode_range_2.write_into(writer);
        self.ul_unicode_range_3.write_into(writer);
        self.ul_unicode_range_4.write_into(writer);
        self.ach_vend_id.write_into(writer);
        self.fs_selection.write_into(writer);
        self.us_first_char_index.write_into(writer);
        self.us_last_char_index.write_into(writer);
        self.s_typo_ascender.write_into(writer);
        self.s_typo_descender.write_into(writer);
        self.s_typo_line_gap.write_into(writer);
        self.us_win_ascent.write_into(writer);
        self.us_win_descent.write_into(writer);
        version.compatible(1u16).then(|| {
            self.ul_code_page_range_1
                .as_ref()
                .expect("missing versioned field should have failed validation")
                .write_into(writer)
        });
        version.compatible(1u16).then(|| {
            self.ul_code_page_range_2
                .as_ref()
                .expect("missing versioned field should have failed validation")
                .write_into(writer)
        });
        version.compatible(2u16).then(|| {
            self.sx_height
                .as_ref()
                .expect("missing versioned field should have failed validation")
                .write_into(writer)
        });
        version.compatible(2u16).then(|| {
            self.s_cap_height
                .as_ref()
                .expect("missing versioned field should have failed validation")
                .write_into(writer)
        });
        version.compatible(2u16).then(|| {
            self.us_default_char
                .as_ref()
                .expect("missing versioned field should have failed validation")
                .write_into(writer)
        });
        version.compatible(2u16).then(|| {
            self.us_break_char
                .as_ref()
                .expect("missing versioned field should have failed validation")
                .write_into(writer)
        });
        version.compatible(2u16).then(|| {
            self.us_max_context
                .as_ref()
                .expect("missing versioned field should have failed validation")
                .write_into(writer)
        });
        version.compatible(5u16).then(|| {
            self.us_lower_optical_point_size
                .as_ref()
                .expect("missing versioned field should have failed validation")
                .write_into(writer)
        });
        version.compatible(5u16).then(|| {
            self.us_upper_optical_point_size
                .as_ref()
                .expect("missing versioned field should have failed validation")
                .write_into(writer)
        });
    }
    fn table_type(&self) -> TableType {
        TableType::TopLevel(Os2::TAG)
    }
}

impl Validate for Os2 {
    fn validate_impl(&self, ctx: &mut ValidationCtx) {
        ctx.in_table("Os2", |ctx| {
            let version: u16 = self.compute_version();
            ctx.in_field("ul_code_page_range_1", |ctx| {
                if version.compatible(1u16) && self.ul_code_page_range_1.is_none() {
                    ctx.report(format!("field must be present for version {version}"));
                }
            });
            ctx.in_field("ul_code_page_range_2", |ctx| {
                if version.compatible(1u16) && self.ul_code_page_range_2.is_none() {
                    ctx.report(format!("field must be present for version {version}"));
                }
            });
            ctx.in_field("sx_height", |ctx| {
                if version.compatible(2u16) && self.sx_height.is_none() {
                    ctx.report(format!("field must be present for version {version}"));
                }
            });
            ctx.in_field("s_cap_height", |ctx| {
                if version.compatible(2u16) && self.s_cap_height.is_none() {
                    ctx.report(format!("field must be present for version {version}"));
                }
            });
            ctx.in_field("us_default_char", |ctx| {
                if version.compatible(2u16) && self.us_default_char.is_none() {
                    ctx.report(format!("field must be present for version {version}"));
                }
            });
            ctx.in_field("us_break_char", |ctx| {
                if version.compatible(2u16) && self.us_break_char.is_none() {
                    ctx.report(format!("field must be present for version {version}"));
                }
            });
            ctx.in_field("us_max_context", |ctx| {
                if version.compatible(2u16) && self.us_max_context.is_none() {
                    ctx.report(format!("field must be present for version {version}"));
                }
            });
            ctx.in_field("us_lower_optical_point_size", |ctx| {
                if version.compatible(5u16) && self.us_lower_optical_point_size.is_none() {
                    ctx.report(format!("field must be present for version {version}"));
                }
            });
            ctx.in_field("us_upper_optical_point_size", |ctx| {
                if version.compatible(5u16) && self.us_upper_optical_point_size.is_none() {
                    ctx.report(format!("field must be present for version {version}"));
                }
            });
        })
    }
}

impl TopLevelTable for Os2 {
    const TAG: Tag = Tag::new(b"OS/2");
}

impl<'a> FromObjRef<read_fonts::tables::os2::Os2<'a>> for Os2 {
    fn from_obj_ref(obj: &read_fonts::tables::os2::Os2<'a>, _: FontData) -> Self {
        Os2 {
            x_avg_char_width: obj.x_avg_char_width(),
            us_weight_class: obj.us_weight_class(),
            us_width_class: obj.us_width_class(),
            fs_type: obj.fs_type(),
            y_subscript_x_size: obj.y_subscript_x_size(),
            y_subscript_y_size: obj.y_subscript_y_size(),
            y_subscript_x_offset: obj.y_subscript_x_offset(),
            y_subscript_y_offset: obj.y_subscript_y_offset(),
            y_superscript_x_size: obj.y_superscript_x_size(),
            y_superscript_y_size: obj.y_superscript_y_size(),
            y_superscript_x_offset: obj.y_superscript_x_offset(),
            y_superscript_y_offset: obj.y_superscript_y_offset(),
            y_strikeout_size: obj.y_strikeout_size(),
            y_strikeout_position: obj.y_strikeout_position(),
            s_family_class: obj.s_family_class(),
            panose_10: convert_panose(obj.panose_10()),
            ul_unicode_range_1: obj.ul_unicode_range_1(),
            ul_unicode_range_2: obj.ul_unicode_range_2(),
            ul_unicode_range_3: obj.ul_unicode_range_3(),
            ul_unicode_range_4: obj.ul_unicode_range_4(),
            ach_vend_id: obj.ach_vend_id(),
            fs_selection: obj.fs_selection(),
            us_first_char_index: obj.us_first_char_index(),
            us_last_char_index: obj.us_last_char_index(),
            s_typo_ascender: obj.s_typo_ascender(),
            s_typo_descender: obj.s_typo_descender(),
            s_typo_line_gap: obj.s_typo_line_gap(),
            us_win_ascent: obj.us_win_ascent(),
            us_win_descent: obj.us_win_descent(),
            ul_code_page_range_1: obj.ul_code_page_range_1(),
            ul_code_page_range_2: obj.ul_code_page_range_2(),
            sx_height: obj.sx_height(),
            s_cap_height: obj.s_cap_height(),
            us_default_char: obj.us_default_char(),
            us_break_char: obj.us_break_char(),
            us_max_context: obj.us_max_context(),
            us_lower_optical_point_size: obj.us_lower_optical_point_size(),
            us_upper_optical_point_size: obj.us_upper_optical_point_size(),
        }
    }
}

impl<'a> FromTableRef<read_fonts::tables::os2::Os2<'a>> for Os2 {}

impl<'a> FontRead<'a> for Os2 {
    fn read(data: FontData<'a>) -> Result<Self, ReadError> {
        <read_fonts::tables::os2::Os2 as FontRead>::read(data).map(|x| x.to_owned_table())
    }
}
