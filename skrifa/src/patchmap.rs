//! Loads incremental font transfer <https://w3c.github.io/IFT/Overview.html> patch mappings.
//!
//! The IFT and IFTX tables encode mappings from subset definitions to URL's which host patches
//! that can be applied to the font to add support for the corresponding subset definition.

use std::collections::BTreeSet;

use crate::Tag;
use raw::FontData;
use read_fonts::{
    tables::ift::{EntryMapRecord, Ift, PatchMapFormat1},
    ReadError, TableProvider,
};

use read_fonts::collections::IntSet;

use crate::charmap::Charmap;

/// Find the set of patches which intersect the specified subset definition.
pub fn intersecting_patches<'a>(
    font: &impl TableProvider<'a>,
    codepoints: &IntSet<u32>,
    features: &BTreeSet<Tag>,
) -> Result<Vec<PatchUri>, ReadError> {
    // TODO(garretrieger): move this function to a struct so we can optionally store
    //  indexes or other data to accelerate intersection.
    let mut result: Vec<PatchUri> = vec![];
    if let Ok(ift) = font.ift() {
        add_intersecting_patches(font, &ift, codepoints, features, &mut result)?;
    };
    if let Ok(iftx) = font.iftx() {
        add_intersecting_patches(font, &iftx, codepoints, features, &mut result)?;
    };

    Ok(result)
}

fn add_intersecting_patches<'a>(
    font: &impl TableProvider<'a>,
    ift: &Ift,
    codepoints: &IntSet<u32>,
    features: &BTreeSet<Tag>,
    patches: &mut Vec<PatchUri>,
) -> Result<(), ReadError> {
    match ift {
        Ift::Format1(format_1) => {
            add_intersecting_format1_patches(font, &format_1, codepoints, features, patches)
        }
        Ift::Format2(_) => todo!(),
    }
}

fn add_intersecting_format1_patches<'a>(
    font: &impl TableProvider<'a>,
    map: &PatchMapFormat1,
    codepoints: &IntSet<u32>,
    features: &BTreeSet<Tag>, // TODO(garretrieger): verify tag sorting matches specification description.
    patches: &mut Vec<PatchUri>, // TODO(garretrieger): btree set to allow for de-duping?
) -> Result<(), ReadError> {
    // Step 0: Top Level Field Validation
    if let Ok(maxp) = font.maxp() {
        if map.glyph_count() != maxp.num_glyphs() as u32 {
            return Err(ReadError::MalformedData(
                "IFT glyph count must match maxp glyph count.",
            ));
        }
    }

    let max_entry_index = map.max_entry_index();
    let max_glyph_map_entry_index = map.max_glyph_map_entry_index();
    if max_glyph_map_entry_index > max_entry_index {
        return Err(ReadError::MalformedData(
            "max_glyph_map_entry_index() must be >= max_entry_index().",
        ));
    }

    let Ok(uri_template) = map.uri_template_as_string() else {
        return Err(ReadError::MalformedData(
            "Invalid unicode string for the uri_template.",
        ));
    };

    let Some(encoding) = PatchEncoding::from_format_number(map.patch_encoding()) else {
        return Err(ReadError::MalformedData(
            "Unrecognized patch encoding format number.",
        ));
    };

    // Step 1: Collect the glyph map entries.
    let mut entries = IntSet::<u16>::empty();
    intersect_format1_glyph_map(font, map, codepoints, &mut entries)?;

    // Step 2: Collect feature mappings.
    intersect_format1_feature_map(map, features, &mut entries)?;

    // Step 3: produce final output.
    patches.extend(
        entries
            .iter()
            // Entry 0 is the entry for codepoints already in the font, so it's always considered applied and skipped.
            .filter(|index| *index > 0)
            .filter(|index| !map.is_entry_applied(*index))
            .map(|index| PatchUri::from_index(uri_template, index as u32, encoding)),
    );
    Ok(())
}

fn intersect_format1_glyph_map<'a>(
    font: &impl TableProvider<'a>,
    map: &PatchMapFormat1,
    codepoints: &IntSet<u32>,
    entries: &mut IntSet<u16>,
) -> Result<(), ReadError> {
    let charmap = Charmap::new(font);
    let glyph_map = map.glyph_map()?;
    let first_gid = glyph_map.first_mapped_glyph() as u32;
    let max_glyph_map_entry_index = map.max_glyph_map_entry_index();

    // TODO(garretrieger): special case codepoints = * (inverted set) and large codepoints sets
    //   produce the codepoint set to be processed by walking the cmap mapping and filtering against he input set.
    for cp in codepoints.iter() {
        // TODO(garretrieger): since codepoints are looked up in sorted order we may be able to speed up the charmap lookup
        // (eg. walking the charmap in parallel with the codepoints, or caching the last binary search index)
        let Some(gid) = charmap.map(cp) else {
            continue;
        };

        let entry_index = if gid.to_u32() < first_gid {
            0
        } else {
            glyph_map
                .entry_index()
                .get((gid.to_u32() - first_gid) as usize)?
                .get()
        };

        if entry_index > max_glyph_map_entry_index {
            continue;
        }

        entries.insert(entry_index);
    }

    Ok(())
}

fn intersect_format1_feature_map<'a>(
    map: &PatchMapFormat1,
    features: &BTreeSet<Tag>,
    entries: &mut IntSet<u16>,
) -> Result<(), ReadError> {
    // TODO(garretrieger): special case features = * (inverted set)
    let max_entry_index = map.max_entry_index();
    let max_glyph_map_entry_index = map.max_glyph_map_entry_index();
    let field_width = if max_entry_index < 256 { 1 } else { 2 };
    if let Some(feature_map) = map.feature_map() {
        // TODO(garretrieger): validate there is enough data for entryMapRecords.
        let feature_map = feature_map?;
        let mut tag_it = features.iter();
        let mut record_it = feature_map.feature_records().iter();

        let mut next_tag = tag_it.next();
        let mut next_record = record_it.next();
        let mut cumulative_entry_map_count = 0;
        let mut largest_tag: Option<Tag> = None;
        loop {
            let (Some(tag), Some(record)) = (next_tag, next_record.as_ref()) else {
                break;
            };
            let record = match record {
                Ok(record) => record,
                Err(err) => return Err(err.clone()),
            };

            if *tag > record.feature_tag() {
                next_record = record_it.next();
                continue;
            }

            if let Some(largest_tag) = largest_tag {
                if *tag <= largest_tag {
                    // Out of order or duplicate tag, skip this record.
                    next_tag = tag_it.next();
                    continue;
                }
            }

            largest_tag = Some(*tag);

            let entry_count = record.entry_map_count().get();
            let tot_entry_count = cumulative_entry_map_count;
            cumulative_entry_map_count += entry_count;
            if *tag < record.feature_tag() {
                next_tag = tag_it.next();
                continue;
            }

            for i in 0..entry_count {
                let index = i + tot_entry_count;
                let byte_index = (index * field_width * 2) as usize;
                let data = FontData::new(&feature_map.entry_map_data()[byte_index..]);
                let record = EntryMapRecord::read(data, max_entry_index)?;
                let mapped_entry_index = record.first_entry_index().get() + i;
                let first = record.first_entry_index().get();
                let last = record.first_entry_index().get();
                if first > last
                    || first > max_glyph_map_entry_index
                    || last > max_glyph_map_entry_index
                    || mapped_entry_index <= max_glyph_map_entry_index
                    || mapped_entry_index > max_entry_index
                {
                    // Invalid, continue on
                    continue;
                }

                entries.insert(mapped_entry_index);
            }
        }
    }
    Ok(())
}

#[derive(Clone, Eq, PartialEq, Debug, Hash, Copy)]
pub enum PatchEncoding {
    Brotli,
    PerTableBrotli { fully_invalidating: bool },
    GlyphKeyed,
}

impl PatchEncoding {
    fn from_format_number(format: u8) -> Option<Self> {
        // Based on https://w3c.github.io/IFT/Overview.html#font-patch-formats-summary
        match format {
            1 => Some(Self::Brotli),
            2 => Some(Self::PerTableBrotli {
                fully_invalidating: true,
            }),
            3 => Some(Self::PerTableBrotli {
                fully_invalidating: false,
            }),
            4 => Some(Self::GlyphKeyed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct PatchUri {
    uri: String,
    encoding: PatchEncoding,
}

impl PatchUri {
    fn from_index(uri_template: &str, _entry_index: u32, encoding: PatchEncoding) -> PatchUri {
        PatchUri {
            // TODO(garretrieger): properly implement this, may deserve to go into it's own module.
            uri: uri_template.to_string(),
            encoding,
        }
    }
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct Entry {
    pub patch_uri: PatchUri,
    pub codepoints: IntSet<u32>,
    pub feature_tags: BTreeSet<Tag>,
    pub compatibility_id: [u32; 4],
}

impl std::fmt::Debug for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let values: Vec<_> = self.codepoints.iter().collect();
        write!(f, "Entry({values:?} => {})", self.patch_uri.uri)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use font_test_data as test_data;
    use font_test_data::ift::SIMPLE_FORMAT1;
    use read_fonts::tables::ift::{IFTX_TAG, IFT_TAG};
    use read_fonts::FontRef;
    use write_fonts::FontBuilder;

    fn create_ift_font(font: FontRef, ift: Option<&[u8]>, iftx: Option<&[u8]>) -> Vec<u8> {
        let mut builder = FontBuilder::default();

        if let Some(bytes) = ift {
            builder.add_raw(IFT_TAG, bytes);
        }

        if let Some(bytes) = iftx {
            builder.add_raw(IFTX_TAG, bytes);
        }

        builder.copy_missing_tables(font);
        builder.build()
    }

    // TODO(garretrieger): test immediate rejection of tables that are too short
    //                     (glyph map, feature records, entry map records).
    // TODO(garretrieger): tests for top level rejection criteria.
    // TODO(garretrieger): test w/ multi codepoints mapping to the same glyph.
    // TODO(garretrieger): test w/ IFT + IFTX both populated tables.
    // TODO(garretrieger): test which has entry that has empty codepoint array.
    // TODO(garretrieger): test which has requires URI substitution.
    // TODO(garretrieger): patch encoding lookup + URI substitution.
    // TODO(garretrieger): test with format 1 that has max entry = 0.
    // TODO(garretrieger): fuzzer to check consistency vs intersecting "*" subset def.

    #[test]
    fn format_1_patch_map_u8_entries() {
        let font_bytes = create_ift_font(
            FontRef::new(test_data::ift::IFT_BASE).unwrap(),
            Some(test_data::ift::SIMPLE_FORMAT1),
            None,
        );
        let font = FontRef::new(&font_bytes).unwrap();

        let patches =
            // 0x123 is not in the mapping
            intersecting_patches(&font, &IntSet::from([0x123]), &BTreeSet::<Tag>::from([])).unwrap();
        assert_eq!(Vec::<PatchUri>::new(), patches);

        let patches =
            // 0x13 maps to entry 0
            intersecting_patches(&font, &IntSet::from([0x13]), &BTreeSet::<Tag>::from([])).unwrap();
        assert_eq!(Vec::<PatchUri>::new(), patches);

        let patches =
            // 0x12 maps to entry 1 which is applied
            intersecting_patches(&font, &IntSet::from([0x12]), &BTreeSet::<Tag>::from([])).unwrap();
        assert_eq!(Vec::<PatchUri>::new(), patches);

        let patches =
            // 0x11 maps to entry 2
            intersecting_patches(&font, &IntSet::from([0x11]), &BTreeSet::<Tag>::from([])).unwrap();
        assert_eq!(
            vec![PatchUri {
                uri: "ABCDEFɤ".to_string(),
                encoding: PatchEncoding::GlyphKeyed,
            }],
            patches
        );

        let patches = intersecting_patches(
            &font,
            &IntSet::from([0x11, 0x12, 0x123]),
            &BTreeSet::<Tag>::from([]),
        )
        .unwrap();
        assert_eq!(
            vec![PatchUri {
                uri: "ABCDEFɤ".to_string(),
                encoding: PatchEncoding::GlyphKeyed,
            }],
            patches
        );
    }

    #[test]
    fn format_1_patch_map_glyph_map_too_short() {
        let font_bytes = create_ift_font(
            FontRef::new(test_data::ift::IFT_BASE).unwrap(),
            Some(&test_data::ift::SIMPLE_FORMAT1[..SIMPLE_FORMAT1.len() - 1]),
            None,
        );
        let font = FontRef::new(&font_bytes).unwrap();

        assert!(
            intersecting_patches(&font, &IntSet::from([0x123]), &BTreeSet::<Tag>::from([]))
                .is_err()
        );
    }

    // TODO(garretrieger): feature map too short.
    // TODO(garretrieger): entry map recordds too short.

    #[test]
    fn format_1_patch_map_u16_entries() {
        let font_bytes = create_ift_font(
            FontRef::new(test_data::CMAP12_FONT1).unwrap(),
            Some(test_data::ift::U16_ENTRIES_FORMAT1),
            None,
        );
        let _font = FontRef::new(&font_bytes).unwrap();

        // TODO: once template substituion is available implement this.
    }

    #[test]
    fn format_1_patch_map_u16_entries_with_feature_mapping() {
        let font_bytes = create_ift_font(
            FontRef::new(test_data::CMAP12_FONT1).unwrap(),
            Some(test_data::ift::FEATURE_MAP_FORMAT1),
            None,
        );
        let _font = FontRef::new(&font_bytes).unwrap();

        // TODO: once template substituion is available implement this.
    }
}
