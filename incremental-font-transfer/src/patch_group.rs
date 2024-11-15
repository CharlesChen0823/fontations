//! API for selecting and applying a group of IFT patches.
//!
//! This provides methods for selecting a maximal group of patches that are compatible with each other and
//! additionally methods for applying that group of patches.

use read_fonts::{tables::ift::CompatibilityId, FontRef, ReadError, TableProvider};
use std::collections::{BTreeMap, HashMap};

use crate::{
    font_patch::{IncrementalFontPatchBase, PatchingError},
    patchmap::{intersecting_patches, IftTableTag, PatchEncoding, PatchUri, SubsetDefinition},
};

/// A group of patches derived from a single IFT font.
///
/// This is a group which can be applied simultaneously to that font. Patches are
/// initially missing data which must be fetched and supplied to patch application
/// method.
pub struct PatchGroup<'a> {
    font: FontRef<'a>,
    patches: Option<CompatibleGroup>,
}

impl<'a> PatchGroup<'a> {
    /// Intersect the available and unapplied patches in ift_font against subset_definition
    ///
    /// Returns a group of patches which would be applied next.
    pub fn select_next_patches<'b>(
        ift_font: FontRef<'b>,
        subset_definition: &SubsetDefinition,
    ) -> Result<PatchGroup<'b>, ReadError> {
        let candidates = intersecting_patches(&ift_font, subset_definition)?;
        if candidates.is_empty() {
            return Ok(PatchGroup {
                font: ift_font,
                patches: None,
            });
        }

        let compat_group = Self::select_next_patches_from_candidates(
            candidates,
            ift_font.ift().ok().map(|t| t.compatibility_id()),
            ift_font.iftx().ok().map(|t| t.compatibility_id()),
        )?;

        Ok(PatchGroup {
            font: ift_font,
            patches: Some(compat_group),
        })
    }

    /// Returns an iterator over URIs in this group.
    pub fn uris(&self) -> impl Iterator<Item = &str> {
        self.invalidating_patch_iter()
            .chain(self.non_invalidating_patch_iter())
            .map(|info| info.uri.as_str())
    }

    /// Returns true if there is at least one uri associated with this group.
    pub fn has_uris(&self) -> bool {
        let Some(patches) = &self.patches else {
            return false;
        };
        match patches {
            CompatibleGroup::Full(FullInvalidationPatch(_)) => true,
            CompatibleGroup::Mixed { ift, iftx } => ift.has_uris() || iftx.has_uris(),
        }
    }

    fn next_invalidating_patch(&self) -> Option<&PatchInfo> {
        self.invalidating_patch_iter().next()
    }

    fn invalidating_patch_iter(&self) -> impl Iterator<Item = &PatchInfo> {
        let full = match &self.patches {
            Some(CompatibleGroup::Full(info)) => Some(&info.0),
            _ => None,
        };

        let partial_1 = match &self.patches {
            Some(CompatibleGroup::Mixed {
                ift: ScopedGroup::PartialInvalidation(v),
                iftx: _,
            }) => Some(&v.0),
            _ => None,
        };

        let partial_2 = match &self.patches {
            Some(CompatibleGroup::Mixed {
                ift: _,
                iftx: ScopedGroup::PartialInvalidation(v),
            }) => Some(&v.0),
            _ => None,
        };

        full.into_iter().chain(partial_1).chain(partial_2)
    }

    fn non_invalidating_patch_iter(&self) -> impl Iterator<Item = &PatchInfo> {
        let ift = match &self.patches {
            Some(CompatibleGroup::Mixed { ift, iftx: _ }) => Some(ift),
            _ => None,
        };
        let iftx = match &self.patches {
            Some(CompatibleGroup::Mixed { ift: _, iftx }) => Some(iftx),
            _ => None,
        };

        let it1 = ift
            .into_iter()
            .flat_map(|scope| scope.no_invalidation_iter());
        let it2 = iftx
            .into_iter()
            .flat_map(|scope| scope.no_invalidation_iter());

        it1.chain(it2)
    }

    fn select_next_patches_from_candidates(
        candidates: Vec<PatchUri>,
        ift_compat_id: Option<CompatibilityId>,
        iftx_compat_id: Option<CompatibilityId>,
    ) -> Result<CompatibleGroup, ReadError> {
        // Some notes about this implementation:
        // - From candidates we need to form the largest possible group of patches which follow the selection criteria
        //   from: https://w3c.github.io/IFT/Overview.html#extend-font-subset and won't invalidate each other.
        //
        // - Validation constraints are encoded into the structure of CompatibleGroup so the task here is to fill up
        //   a compatible group appropriately.
        //
        // - When multiple valid choices exist the specification allows the implementation to take one of it's choosing.
        //   Here we use a heuristic that tries to select the patch which has the most value to the extension request.
        //
        // - During selection we need to ensure that there are no PatchInfo's with duplicate URIs. The spec doesn't
        //   require erroring on this case, and it's resolved by:
        //   - In the spec algo patches are selected and applied one at a time.
        //   - Further it specifically disallows re-applying the same URI later.
        //   - So therefore we de-dup by retaining the particular instance which has the highest selection
        //     priority.

        let mut full_invalidation: Vec<FullInvalidationPatch> = vec![];
        let mut partial_invalidation_ift: Vec<PartialInvalidationPatch> = vec![];
        let mut partial_invalidation_iftx: Vec<PartialInvalidationPatch> = vec![];
        // TODO(garretrieger): do we need sorted order, use HashMap instead?
        let mut no_invalidation_ift: BTreeMap<String, NoInvalidationPatch> = Default::default();
        let mut no_invalidation_iftx: BTreeMap<String, NoInvalidationPatch> = Default::default();

        // Step 1: sort the candidates into separate lists based on invalidation characteristics.
        for uri in candidates.into_iter() {
            // TODO(garretrieger): for efficiency can we delay uri template resolution until we have actually selected patches?
            // TODO(garretrieger): for btree construction don't recompute the resolved uri, cache inside the patch uri object?
            match uri.encoding() {
                PatchEncoding::TableKeyed {
                    fully_invalidating: true,
                } => full_invalidation.push(FullInvalidationPatch(uri.into())),
                PatchEncoding::TableKeyed {
                    fully_invalidating: false,
                } => {
                    if Some(uri.expected_compatibility_id()) == ift_compat_id.as_ref() {
                        partial_invalidation_ift.push(PartialInvalidationPatch(uri.into()))
                    } else if Some(uri.expected_compatibility_id()) == iftx_compat_id.as_ref() {
                        partial_invalidation_iftx.push(PartialInvalidationPatch(uri.into()))
                    }
                }
                PatchEncoding::GlyphKeyed => {
                    if Some(uri.expected_compatibility_id()) == ift_compat_id.as_ref() {
                        no_invalidation_ift
                            .insert(uri.uri_string(), NoInvalidationPatch(uri.into()));
                    } else if Some(uri.expected_compatibility_id()) == iftx_compat_id.as_ref() {
                        no_invalidation_iftx
                            .insert(uri.uri_string(), NoInvalidationPatch(uri.into()));
                    }
                }
            }
        }

        // Step 2 - now make patch selections in priority order: first full invalidation, second partial, lastly none.
        if let Some(patch) = full_invalidation.into_iter().next() {
            // TODO(garretrieger): use a heuristic to select the best patch
            return Ok(CompatibleGroup::Full(patch));
        }

        let mut ift_selected_uri: Option<String> = None;
        let ift_scope = partial_invalidation_ift
            .into_iter()
            // TODO(garretrieger): use a heuristic to select the best patch
            .next()
            .map(|patch| {
                ift_selected_uri = Some(patch.0.uri.clone());
                ScopedGroup::PartialInvalidation(patch)
            });

        let mut iftx_selected_uri: Option<String> = None;
        let iftx_scope = partial_invalidation_iftx
            .into_iter()
            .find(|patch| {
                // TODO(garretrieger): use a heuristic to select the best patch
                let Some(selected) = &ift_selected_uri else {
                    return true;
                };
                selected != &patch.0.uri
            })
            .map(|patch| {
                iftx_selected_uri = Some(patch.0.uri.clone());
                ScopedGroup::PartialInvalidation(patch)
            });

        // URI's which have been selected for use above should not show up in other selections.
        if let (Some(uri), None) = (&ift_selected_uri, &iftx_selected_uri) {
            no_invalidation_iftx.remove(uri);
        }
        if let (None, Some(uri)) = (ift_selected_uri, iftx_selected_uri) {
            no_invalidation_ift.remove(&uri);
        }

        match (ift_scope, iftx_scope) {
            (Some(scope1), Some(scope2)) => Ok(CompatibleGroup::Mixed {
                ift: scope1,
                iftx: scope2,
            }),
            (Some(scope1), None) => Ok(CompatibleGroup::Mixed {
                ift: scope1,
                iftx: ScopedGroup::NoInvalidation(no_invalidation_iftx),
            }),
            (None, Some(scope2)) => Ok(CompatibleGroup::Mixed {
                ift: ScopedGroup::NoInvalidation(no_invalidation_ift),
                iftx: scope2,
            }),
            (None, None) => {
                // The two groups can't contain any duplicate URIs so remove all URIs in ift from iftx.
                for uri in no_invalidation_ift.keys() {
                    no_invalidation_iftx.remove(uri);
                }
                Ok(CompatibleGroup::Mixed {
                    ift: ScopedGroup::NoInvalidation(no_invalidation_ift),
                    iftx: ScopedGroup::NoInvalidation(no_invalidation_iftx),
                })
            }
        }
    }

    /// Attempt to apply the next patch (or patches if non-invalidating) listed in this group.
    ///
    /// Returns the bytes of the updated font.
    pub fn apply_next_patches(
        self,
        patch_data: &mut HashMap<String, UriStatus>,
    ) -> Result<Vec<u8>, PatchingError> {
        if let Some(patch) = self.next_invalidating_patch() {
            let entry = patch_data
                .get_mut(&patch.uri)
                .ok_or(PatchingError::MissingPatches)?;

            match entry {
                UriStatus::Pending(patch_data) => {
                    let r = self.font.apply_table_keyed_patch(patch, patch_data)?;
                    *entry = UriStatus::Applied;
                    return Ok(r);
                }
                UriStatus::Applied => {} // previously applied uris are ignored according to the spec.
            }
        }

        // No invalidating patches left, so apply any non invalidating ones in one pass.
        // First check if we have all of the needed data.
        let new_font = {
            let mut accumulated_info: Vec<(&PatchInfo, &[u8])> = vec![];
            for info in self.non_invalidating_patch_iter() {
                let data = patch_data
                    .get(&info.uri)
                    .ok_or(PatchingError::MissingPatches)?;

                match data {
                    UriStatus::Pending(data) => accumulated_info.push((info, data)),
                    UriStatus::Applied => {} // previously applied uris are ignored according to the spec.
                }
            }

            if accumulated_info.is_empty() {
                return Err(PatchingError::EmptyPatchList);
            }

            self.font
                .apply_glyph_keyed_patches(accumulated_info.into_iter())?
        };

        for info in self.non_invalidating_patch_iter() {
            if let Some(status) = patch_data.get_mut(&info.uri) {
                *status = UriStatus::Applied;
            };
        }

        Ok(new_font)
    }
}

/// Tracks whether a URI has already been applied to a font or not.
#[derive(PartialEq, Eq, Debug)]
pub enum UriStatus {
    Applied,
    Pending(Vec<u8>),
}

/// Tracks information related to a patch necessary to apply that patch.
#[derive(PartialEq, Eq, Debug)]
pub(crate) struct PatchInfo {
    uri: String,
    source_table: IftTableTag,
    // TODO: details for how to mark the patch applied in the mapping table (ie. bit index to flip).
    // TODO: Signals for heuristic patch selection:
}

impl PatchInfo {
    pub(crate) fn tag(&self) -> &IftTableTag {
        &self.source_table
    }
}

impl From<PatchUri> for PatchInfo {
    fn from(value: PatchUri) -> Self {
        PatchInfo {
            uri: value.uri_string(),
            source_table: value.source_table(),
        }
    }
}

/// Type for a single non invalidating patch.
#[derive(PartialEq, Eq, Debug)]
struct NoInvalidationPatch(PatchInfo);

/// Type for a single partially invalidating patch.
#[derive(PartialEq, Eq, Debug)]
struct PartialInvalidationPatch(PatchInfo);

/// Type for a single fully invalidating patch.
#[derive(PartialEq, Eq, Debug)]
struct FullInvalidationPatch(PatchInfo);

/// Represents a group of patches which are valid (compatible) to be applied together to
/// an IFT font.
#[derive(PartialEq, Eq, Debug)]
enum CompatibleGroup {
    Full(FullInvalidationPatch),
    Mixed { ift: ScopedGroup, iftx: ScopedGroup },
}

/// A set of zero or more compatible patches that are derived from the same scope
/// ("IFT " vs "IFTX")
#[derive(PartialEq, Eq, Debug)]
enum ScopedGroup {
    PartialInvalidation(PartialInvalidationPatch),
    NoInvalidation(BTreeMap<String, NoInvalidationPatch>),
}

impl ScopedGroup {
    fn has_uris(&self) -> bool {
        match self {
            ScopedGroup::PartialInvalidation(PartialInvalidationPatch(_)) => true,
            ScopedGroup::NoInvalidation(uri_map) => !uri_map.is_empty(),
        }
    }

    fn no_invalidation_iter(&self) -> impl Iterator<Item = &PatchInfo> {
        match self {
            ScopedGroup::PartialInvalidation(_) => NoInvalidationPatchesIter { it: None },
            ScopedGroup::NoInvalidation(map) => NoInvalidationPatchesIter {
                it: Some(map.values()),
            },
        }
    }
}

struct NoInvalidationPatchesIter<'a, T>
where
    T: Iterator<Item = &'a NoInvalidationPatch>,
{
    it: Option<T>,
}

impl<'a, T> Iterator for NoInvalidationPatchesIter<'a, T>
where
    T: Iterator<Item = &'a NoInvalidationPatch>,
{
    type Item = &'a PatchInfo;

    fn next(&mut self) -> Option<Self::Item> {
        let it = self.it.as_mut()?;
        Some(&it.next()?.0)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::glyph_keyed::tests::assemble_glyph_keyed_patch;
    use font_test_data::ift::{
        glyf_u16_glyph_patches, glyph_keyed_patch_header, table_keyed_format2, table_keyed_patch,
        test_font_for_patching_with_loca_mod,
    };

    use font_types::{Int24, Tag};

    use read_fonts::{test_helpers::BeBuffer, FontRef};

    use write_fonts::FontBuilder;

    const TABLE_1_FINAL_STATE: &[u8] = "hijkabcdeflmnohijkabcdeflmno\n".as_bytes();
    const TABLE_2_FINAL_STATE: &[u8] = "foobarbaz foobarbaz foobarbaz\n".as_bytes();

    fn base_font(ift: Option<BeBuffer>, iftx: Option<BeBuffer>) -> Vec<u8> {
        let mut font_builder = FontBuilder::new();

        if let Some(buffer) = &ift {
            font_builder.add_raw(Tag::new(b"IFT "), buffer.as_slice());
        }
        if let Some(buffer) = &iftx {
            font_builder.add_raw(Tag::new(b"IFTX"), buffer.as_slice());
        }

        font_builder.add_raw(Tag::new(b"tab1"), "abcdef\n".as_bytes());
        font_builder.add_raw(Tag::new(b"tab2"), "foobar\n".as_bytes());
        font_builder.add_raw(Tag::new(b"tab4"), "abcdef\n".as_bytes());
        font_builder.add_raw(Tag::new(b"tab5"), "foobar\n".as_bytes());
        font_builder.build()
    }

    fn cid_1() -> CompatibilityId {
        CompatibilityId::from_u32s([0, 0, 0, 1])
    }

    fn cid_2() -> CompatibilityId {
        CompatibilityId::from_u32s([0, 0, 0, 2])
    }

    fn p1_full() -> PatchUri {
        PatchUri::from_index(
            "//foo.bar/{id}",
            1,
            &IftTableTag::Ift(cid_1()),
            PatchEncoding::TableKeyed {
                fully_invalidating: true,
            },
        )
    }

    fn p2_partial_c1() -> PatchUri {
        PatchUri::from_index(
            "//foo.bar/{id}",
            2,
            &IftTableTag::Ift(cid_1()),
            PatchEncoding::TableKeyed {
                fully_invalidating: false,
            },
        )
    }

    fn p2_partial_c2() -> PatchUri {
        PatchUri::from_index(
            "//foo.bar/{id}",
            2,
            &IftTableTag::Iftx(cid_2()),
            PatchEncoding::TableKeyed {
                fully_invalidating: false,
            },
        )
    }

    fn p2_no_c2() -> PatchUri {
        PatchUri::from_index(
            "//foo.bar/{id}",
            2,
            &IftTableTag::Iftx(cid_2()),
            PatchEncoding::GlyphKeyed,
        )
    }

    fn p2_partial_c2_ift() -> PatchUri {
        PatchUri::from_index(
            "//foo.bar/{id}",
            2,
            &IftTableTag::Ift(cid_2()),
            PatchEncoding::TableKeyed {
                fully_invalidating: false,
            },
        )
    }

    fn p3_partial_c2() -> PatchUri {
        PatchUri::from_index(
            "//foo.bar/{id}",
            3,
            &IftTableTag::Iftx(cid_2()),
            PatchEncoding::TableKeyed {
                fully_invalidating: false,
            },
        )
    }

    fn p3_no_c1() -> PatchUri {
        PatchUri::from_index(
            "//foo.bar/{id}",
            3,
            &IftTableTag::Ift(cid_1()),
            PatchEncoding::GlyphKeyed,
        )
    }

    fn p4_no_c1() -> PatchUri {
        PatchUri::from_index(
            "//foo.bar/{id}",
            4,
            &IftTableTag::Ift(cid_1()),
            PatchEncoding::GlyphKeyed,
        )
    }

    fn p4_no_c2() -> PatchUri {
        PatchUri::from_index(
            "//foo.bar/{id}",
            4,
            &IftTableTag::Iftx(cid_2()),
            PatchEncoding::GlyphKeyed,
        )
    }

    fn p5_no_c2() -> PatchUri {
        PatchUri::from_index(
            "//foo.bar/{id}",
            5,
            &IftTableTag::Iftx(cid_2()),
            PatchEncoding::GlyphKeyed,
        )
    }

    fn patch_info_ift(uri: &str) -> PatchInfo {
        PatchInfo {
            uri: uri.to_string(),
            source_table: IftTableTag::Ift(cid_1()),
        }
    }

    fn patch_info_ift_c2(uri: &str) -> PatchInfo {
        PatchInfo {
            uri: uri.to_string(),
            source_table: IftTableTag::Ift(cid_2()),
        }
    }

    fn patch_info_iftx(uri: &str) -> PatchInfo {
        PatchInfo {
            uri: uri.to_string(),
            source_table: IftTableTag::Iftx(cid_2()),
        }
    }

    #[test]
    fn full_invalidation() {
        let group = PatchGroup::select_next_patches_from_candidates(
            vec![p1_full()],
            Some(cid_1()),
            Some(cid_2()),
        )
        .unwrap();

        assert_eq!(
            group,
            CompatibleGroup::Full(FullInvalidationPatch(patch_info_ift("//foo.bar/04")))
        );

        let group = PatchGroup::select_next_patches_from_candidates(
            vec![
                p1_full(),
                p2_partial_c1(),
                p3_partial_c2(),
                p4_no_c1(),
                p5_no_c2(),
            ],
            Some(cid_1()),
            Some(cid_2()),
        )
        .unwrap();

        assert_eq!(
            group,
            CompatibleGroup::Full(FullInvalidationPatch(patch_info_ift("//foo.bar/04"),))
        );
    }

    #[test]
    fn mixed() {
        // (partial, no inval)
        let group = PatchGroup::select_next_patches_from_candidates(
            vec![p2_partial_c1(), p4_no_c1(), p5_no_c2()],
            Some(cid_1()),
            Some(cid_2()),
        )
        .unwrap();

        assert_eq!(
            group,
            CompatibleGroup::Mixed {
                ift: ScopedGroup::PartialInvalidation(PartialInvalidationPatch(patch_info_ift(
                    "//foo.bar/08"
                ),)),
                iftx: ScopedGroup::NoInvalidation(BTreeMap::from([(
                    "//foo.bar/0K".to_string(),
                    NoInvalidationPatch(patch_info_iftx("//foo.bar/0K"))
                )]))
            }
        );

        // (no inval, partial)
        let group = PatchGroup::select_next_patches_from_candidates(
            vec![p3_partial_c2(), p4_no_c1(), p5_no_c2()],
            Some(cid_1()),
            Some(cid_2()),
        )
        .unwrap();

        assert_eq!(
            group,
            CompatibleGroup::Mixed {
                ift: ScopedGroup::NoInvalidation(BTreeMap::from([(
                    "//foo.bar/0G".to_string(),
                    NoInvalidationPatch(patch_info_ift("//foo.bar/0G"))
                )])),
                iftx: ScopedGroup::PartialInvalidation(PartialInvalidationPatch(patch_info_iftx(
                    "//foo.bar/0C"
                ),))
            }
        );

        // (partial, empty)
        let group = PatchGroup::select_next_patches_from_candidates(
            vec![p2_partial_c1(), p4_no_c1()],
            Some(cid_1()),
            Some(cid_2()),
        )
        .unwrap();

        assert_eq!(
            group,
            CompatibleGroup::Mixed {
                ift: ScopedGroup::PartialInvalidation(PartialInvalidationPatch(patch_info_ift(
                    "//foo.bar/08"
                ),)),
                iftx: ScopedGroup::NoInvalidation(BTreeMap::default()),
            }
        );

        // (empty, partial)
        let group = PatchGroup::select_next_patches_from_candidates(
            vec![p3_partial_c2(), p5_no_c2()],
            Some(cid_1()),
            Some(cid_2()),
        )
        .unwrap();

        assert_eq!(
            group,
            CompatibleGroup::Mixed {
                ift: ScopedGroup::NoInvalidation(BTreeMap::default()),
                iftx: ScopedGroup::PartialInvalidation(PartialInvalidationPatch(patch_info_iftx(
                    "//foo.bar/0C"
                ),)),
            }
        );
    }

    #[test]
    fn missing_compat_ids() {
        // (None, None)
        let group = PatchGroup::select_next_patches_from_candidates(
            vec![p2_partial_c1(), p4_no_c1(), p5_no_c2()],
            None,
            None,
        )
        .unwrap();

        assert_eq!(
            group,
            CompatibleGroup::Mixed {
                ift: ScopedGroup::NoInvalidation(Default::default()),
                iftx: ScopedGroup::NoInvalidation(Default::default()),
            }
        );

        // (Some, None)
        let group = PatchGroup::select_next_patches_from_candidates(
            vec![p2_partial_c1(), p4_no_c1(), p5_no_c2()],
            Some(cid_1()),
            None,
        )
        .unwrap();

        assert_eq!(
            group,
            CompatibleGroup::Mixed {
                ift: ScopedGroup::PartialInvalidation(PartialInvalidationPatch(patch_info_ift(
                    "//foo.bar/08"
                ),)),
                iftx: ScopedGroup::NoInvalidation(Default::default()),
            }
        );

        // (None, Some)
        let group = PatchGroup::select_next_patches_from_candidates(
            vec![p2_partial_c1(), p4_no_c1(), p5_no_c2()],
            None,
            Some(cid_1()),
        )
        .unwrap();

        assert_eq!(
            group,
            CompatibleGroup::Mixed {
                ift: ScopedGroup::NoInvalidation(Default::default()),
                iftx: ScopedGroup::PartialInvalidation(PartialInvalidationPatch(patch_info_ift(
                    "//foo.bar/08"
                ),)),
            }
        );
    }

    #[test]
    fn tables_have_same_compat_id() {
        let group = PatchGroup::select_next_patches_from_candidates(
            vec![
                p2_partial_c1(),
                p2_partial_c2_ift(),
                p3_partial_c2(),
                p4_no_c1(),
                p5_no_c2(),
            ],
            Some(cid_2()),
            Some(cid_2()),
        )
        .unwrap();

        assert_eq!(
            group,
            CompatibleGroup::Mixed {
                ift: ScopedGroup::PartialInvalidation(PartialInvalidationPatch(patch_info_ift_c2(
                    "//foo.bar/08"
                ),)),
                iftx: ScopedGroup::NoInvalidation(BTreeMap::new()),
            }
        );

        // Check that input order determines the winner.
        let group = PatchGroup::select_next_patches_from_candidates(
            vec![
                p2_partial_c1(),
                p3_partial_c2(),
                p2_partial_c2_ift(),
                p4_no_c1(),
                p5_no_c2(),
            ],
            Some(cid_2()),
            Some(cid_2()),
        )
        .unwrap();

        assert_eq!(
            group,
            CompatibleGroup::Mixed {
                ift: ScopedGroup::PartialInvalidation(PartialInvalidationPatch(patch_info_iftx(
                    "//foo.bar/0C"
                ),)),
                iftx: ScopedGroup::NoInvalidation(BTreeMap::new()),
            }
        );
    }

    #[test]
    fn dedups_uris() {
        // Duplicates inside a scope
        let group = PatchGroup::select_next_patches_from_candidates(
            vec![p4_no_c1(), p4_no_c1()],
            Some(cid_1()),
            Some(cid_2()),
        )
        .unwrap();

        assert_eq!(
            group,
            CompatibleGroup::Mixed {
                ift: ScopedGroup::NoInvalidation(BTreeMap::from([(
                    "//foo.bar/0G".to_string(),
                    NoInvalidationPatch(patch_info_ift("//foo.bar/0G"))
                )])),
                iftx: ScopedGroup::NoInvalidation(BTreeMap::new()),
            }
        );

        // Duplicates across scopes (no invalidation + no invalidation)
        let group = PatchGroup::select_next_patches_from_candidates(
            vec![p4_no_c1(), p4_no_c2(), p5_no_c2()],
            Some(cid_1()),
            Some(cid_2()),
        )
        .unwrap();

        assert_eq!(
            group,
            CompatibleGroup::Mixed {
                ift: ScopedGroup::NoInvalidation(BTreeMap::from([(
                    "//foo.bar/0G".to_string(),
                    NoInvalidationPatch(patch_info_ift("//foo.bar/0G"))
                )])),
                iftx: ScopedGroup::NoInvalidation(BTreeMap::from([(
                    "//foo.bar/0K".to_string(),
                    NoInvalidationPatch(patch_info_iftx("//foo.bar/0K"))
                )])),
            }
        );

        // Duplicates across scopes (partial + partial)
        let group = PatchGroup::select_next_patches_from_candidates(
            vec![p2_partial_c1(), p2_partial_c2(), p3_partial_c2()],
            Some(cid_1()),
            Some(cid_2()),
        )
        .unwrap();

        assert_eq!(
            group,
            CompatibleGroup::Mixed {
                ift: ScopedGroup::PartialInvalidation(PartialInvalidationPatch(patch_info_ift(
                    "//foo.bar/08"
                ))),
                iftx: ScopedGroup::PartialInvalidation(PartialInvalidationPatch(patch_info_iftx(
                    "//foo.bar/0C"
                ))),
            }
        );

        // Duplicates across scopes (partial + no invalidation)
        let group = PatchGroup::select_next_patches_from_candidates(
            vec![p2_partial_c1(), p2_no_c2(), p5_no_c2()],
            Some(cid_1()),
            Some(cid_2()),
        )
        .unwrap();

        assert_eq!(
            group,
            CompatibleGroup::Mixed {
                ift: ScopedGroup::PartialInvalidation(PartialInvalidationPatch(patch_info_ift(
                    "//foo.bar/08"
                ))),
                iftx: ScopedGroup::NoInvalidation(BTreeMap::from([(
                    "//foo.bar/0K".to_string(),
                    NoInvalidationPatch(patch_info_iftx("//foo.bar/0K"))
                )])),
            }
        );

        let group = PatchGroup::select_next_patches_from_candidates(
            vec![p3_partial_c2(), p3_no_c1(), p4_no_c1()],
            Some(cid_1()),
            Some(cid_2()),
        )
        .unwrap();

        assert_eq!(
            group,
            CompatibleGroup::Mixed {
                ift: ScopedGroup::NoInvalidation(BTreeMap::from([(
                    "//foo.bar/0G".to_string(),
                    NoInvalidationPatch(patch_info_ift("//foo.bar/0G"))
                )])),
                iftx: ScopedGroup::PartialInvalidation(PartialInvalidationPatch(patch_info_iftx(
                    "//foo.bar/0C"
                ))),
            }
        );
    }

    fn create_group_for(uris: Vec<PatchUri>) -> PatchGroup<'static> {
        let data = FontRef::new(font_test_data::CMAP12_FONT1).unwrap();
        let group =
            PatchGroup::select_next_patches_from_candidates(uris, Some(cid_1()), Some(cid_2()))
                .unwrap();

        PatchGroup {
            font: data,
            patches: Some(group),
        }
    }

    fn empty_group() -> PatchGroup<'static> {
        let data = FontRef::new(font_test_data::CMAP12_FONT1).unwrap();
        PatchGroup {
            font: data,
            patches: None,
        }
    }

    #[test]
    fn uris() {
        let g = create_group_for(vec![]);
        assert_eq!(g.uris().collect::<Vec<&str>>(), Vec::<&str>::default());
        assert!(!g.has_uris());

        let g = empty_group();
        assert_eq!(g.uris().collect::<Vec<&str>>(), Vec::<&str>::default());
        assert!(!g.has_uris());

        let g = create_group_for(vec![p1_full()]);
        assert_eq!(g.uris().collect::<Vec<&str>>(), vec!["//foo.bar/04"],);
        assert!(g.has_uris());

        let g = create_group_for(vec![p2_partial_c1(), p3_partial_c2()]);
        assert_eq!(
            g.uris().collect::<Vec<&str>>(),
            vec!["//foo.bar/08", "//foo.bar/0C"]
        );
        assert!(g.has_uris());

        let g = create_group_for(vec![p2_partial_c1()]);
        assert_eq!(g.uris().collect::<Vec<&str>>(), vec!["//foo.bar/08",],);
        assert!(g.has_uris());

        let g = create_group_for(vec![p3_partial_c2()]);
        assert_eq!(g.uris().collect::<Vec<&str>>(), vec!["//foo.bar/0C"],);
        assert!(g.has_uris());

        let g = create_group_for(vec![p2_partial_c1(), p4_no_c2(), p5_no_c2()]);
        assert_eq!(
            g.uris().collect::<Vec<&str>>(),
            vec!["//foo.bar/08", "//foo.bar/0G", "//foo.bar/0K"],
        );
        assert!(g.has_uris());

        let g = create_group_for(vec![p3_partial_c2(), p4_no_c1()]);
        assert_eq!(
            g.uris().collect::<Vec<&str>>(),
            vec!["//foo.bar/0C", "//foo.bar/0G"],
        );

        let g = create_group_for(vec![p4_no_c1(), p5_no_c2()]);
        assert_eq!(
            g.uris().collect::<Vec<&str>>(),
            vec!["//foo.bar/0G", "//foo.bar/0K"],
        );
        assert!(g.has_uris());
    }

    #[test]
    fn select_next_patches_no_intersection() {
        let font = base_font(Some(table_keyed_format2()), None);
        let font = FontRef::new(&font).unwrap();

        let s = SubsetDefinition::codepoints([55].into_iter().collect());
        let g = PatchGroup::select_next_patches(font, &s).unwrap();

        assert!(!g.has_uris());
        assert_eq!(g.uris().collect::<Vec<&str>>(), Vec::<&str>::default());

        assert_eq!(
            g.apply_next_patches(&mut Default::default()),
            Err(PatchingError::EmptyPatchList)
        );
    }

    #[test]
    fn apply_patches_full_invalidation() {
        let font = base_font(Some(table_keyed_format2()), None);
        let font = FontRef::new(&font).unwrap();

        let s = SubsetDefinition::codepoints([5].into_iter().collect());
        let g = PatchGroup::select_next_patches(font, &s).unwrap();

        assert!(g.has_uris());
        let mut patch_data = HashMap::from([
            (
                "foo/04".to_string(),
                UriStatus::Pending(table_keyed_patch().as_slice().to_vec()),
            ),
            (
                "foo/bar".to_string(),
                UriStatus::Pending(table_keyed_patch().as_slice().to_vec()),
            ),
        ]);

        let new_font = g.apply_next_patches(&mut patch_data).unwrap();
        let new_font = FontRef::new(&new_font).unwrap();

        assert_eq!(
            new_font.table_data(Tag::new(b"tab1")).unwrap().as_bytes(),
            TABLE_1_FINAL_STATE,
        );
        assert_eq!(
            new_font.table_data(Tag::new(b"tab2")).unwrap().as_bytes(),
            TABLE_2_FINAL_STATE,
        );

        assert_eq!(
            patch_data,
            HashMap::from([
                ("foo/04".to_string(), UriStatus::Applied,),
                (
                    "foo/bar".to_string(),
                    UriStatus::Pending(table_keyed_patch().as_slice().to_vec()),
                ),
            ])
        )
    }

    #[test]
    fn apply_patches_one_partial_invalidation() {
        let mut buffer = table_keyed_format2();
        buffer.write_at("encoding", 2u8);

        // IFT
        let font = base_font(Some(buffer.clone()), None);
        let font = FontRef::new(&font).unwrap();

        let s = SubsetDefinition::codepoints([5].into_iter().collect());
        let g = PatchGroup::select_next_patches(font, &s).unwrap();

        let mut patch_data = HashMap::from([(
            "foo/04".to_string(),
            UriStatus::Pending(table_keyed_patch().as_slice().to_vec()),
        )]);

        let new_font = g.apply_next_patches(&mut patch_data).unwrap();
        let new_font = FontRef::new(&new_font).unwrap();

        assert_eq!(
            new_font.table_data(Tag::new(b"tab1")).unwrap().as_bytes(),
            TABLE_1_FINAL_STATE,
        );
        assert_eq!(
            new_font.table_data(Tag::new(b"tab2")).unwrap().as_bytes(),
            TABLE_2_FINAL_STATE,
        );

        assert_eq!(
            patch_data,
            HashMap::from([("foo/04".to_string(), UriStatus::Applied,),])
        );

        // IFTX
        let font = base_font(None, Some(buffer.clone()));
        let font = FontRef::new(&font).unwrap();

        let s = SubsetDefinition::codepoints([5].into_iter().collect());
        let g = PatchGroup::select_next_patches(font, &s).unwrap();

        let mut patch_data = HashMap::from([(
            "foo/04".to_string(),
            UriStatus::Pending(table_keyed_patch().as_slice().to_vec()),
        )]);

        let new_font = g.apply_next_patches(&mut patch_data).unwrap();
        let new_font = FontRef::new(&new_font).unwrap();

        assert_eq!(
            new_font.table_data(Tag::new(b"tab1")).unwrap().as_bytes(),
            TABLE_1_FINAL_STATE,
        );
        assert_eq!(
            new_font.table_data(Tag::new(b"tab2")).unwrap().as_bytes(),
            TABLE_2_FINAL_STATE,
        );

        assert_eq!(
            patch_data,
            HashMap::from([("foo/04".to_string(), UriStatus::Applied,),])
        );
    }

    #[test]
    fn apply_patches_two_partial_invalidation() {
        let mut ift_buffer = table_keyed_format2();
        ift_buffer.write_at("encoding", 2u8);

        let mut iftx_buffer = table_keyed_format2();
        iftx_buffer.write_at("compat_id[0]", 2u32);
        iftx_buffer.write_at("encoding", 2u8);
        iftx_buffer.write_at("id_delta", Int24::new(1));

        let font = base_font(Some(ift_buffer), Some(iftx_buffer));
        let font = FontRef::new(&font).unwrap();

        let s = SubsetDefinition::codepoints([5].into_iter().collect());
        let g = PatchGroup::select_next_patches(font.clone(), &s).unwrap();

        let mut patch_2 = table_keyed_patch();
        patch_2.write_at("compat_id", 2u32);
        patch_2.write_at("patch[0]", Tag::new(b"tab4"));
        patch_2.write_at("patch[1]", Tag::new(b"tab5"));

        let mut patch_data = HashMap::from([
            (
                "foo/04".to_string(),
                UriStatus::Pending(table_keyed_patch().as_slice().to_vec()),
            ),
            (
                "foo/08".to_string(),
                UriStatus::Pending(patch_2.as_slice().to_vec()),
            ),
        ]);

        let new_font = g.apply_next_patches(&mut patch_data).unwrap();
        let new_font = FontRef::new(&new_font).unwrap();

        assert_eq!(
            new_font.table_data(Tag::new(b"tab1")).unwrap().as_bytes(),
            TABLE_1_FINAL_STATE,
        );
        assert_eq!(
            new_font.table_data(Tag::new(b"tab2")).unwrap().as_bytes(),
            TABLE_2_FINAL_STATE,
        );

        // only the first patch gets applied so tab4/tab5 are unchanged.
        assert_eq!(
            new_font.table_data(Tag::new(b"tab4")).unwrap().as_bytes(),
            font.table_data(Tag::new(b"tab4")).unwrap().as_bytes(),
        );
        assert_eq!(
            new_font.table_data(Tag::new(b"tab5")).unwrap().as_bytes(),
            font.table_data(Tag::new(b"tab5")).unwrap().as_bytes(),
        );
    }

    #[test]
    fn apply_patches_mixed() {
        let mut ift_builder = table_keyed_format2();
        ift_builder.write_at("encoding", 2u8);

        let mut iftx_builder = table_keyed_format2();
        iftx_builder.write_at("encoding", 3u8);
        iftx_builder.write_at("compat_id[0]", 6u32);
        iftx_builder.write_at("compat_id[1]", 7u32);
        iftx_builder.write_at("compat_id[2]", 8u32);
        iftx_builder.write_at("compat_id[3]", 9u32);
        iftx_builder.write_at("id_delta", Int24::new(1));

        let font = test_font_for_patching_with_loca_mod(
            |_| {},
            HashMap::from([
                (Tag::new(b"IFT "), ift_builder.as_slice()),
                (Tag::new(b"IFTX"), iftx_builder.as_slice()),
                (Tag::new(b"tab1"), "abcdef\n".as_bytes()),
            ]),
        );
        let font = FontRef::new(font.as_slice()).unwrap();

        let s = SubsetDefinition::codepoints([5].into_iter().collect());
        let g = PatchGroup::select_next_patches(font.clone(), &s).unwrap();

        let patch_ift = table_keyed_patch();
        let patch_iftx =
            assemble_glyph_keyed_patch(glyph_keyed_patch_header(), glyf_u16_glyph_patches());

        let mut patch_data = HashMap::from([
            (
                "foo/04".to_string(),
                UriStatus::Pending(patch_ift.as_slice().to_vec()),
            ),
            (
                "foo/08".to_string(),
                UriStatus::Pending(patch_iftx.as_slice().to_vec()),
            ),
        ]);

        let new_font = g.apply_next_patches(&mut patch_data).unwrap();
        let new_font = FontRef::new(&new_font).unwrap();

        assert_eq!(
            new_font.table_data(Tag::new(b"tab1")).unwrap().as_bytes(),
            TABLE_1_FINAL_STATE,
        );

        // only the partial invalidation patch gets applied, so glyf is unchanged.
        assert_eq!(
            new_font.table_data(Tag::new(b"glyf")).unwrap().as_bytes(),
            font.table_data(Tag::new(b"glyf")).unwrap().as_bytes(),
        );
    }

    #[test]
    fn apply_patches_all_no_invalidation() {
        let mut ift_builder = table_keyed_format2();
        ift_builder.write_at("encoding", 3u8);
        ift_builder.write_at("compat_id[0]", 6u32);
        ift_builder.write_at("compat_id[1]", 7u32);
        ift_builder.write_at("compat_id[2]", 8u32);
        ift_builder.write_at("compat_id[3]", 9u32);

        let mut iftx_builder = table_keyed_format2();
        iftx_builder.write_at("encoding", 3u8);
        iftx_builder.write_at("compat_id[0]", 6u32);
        iftx_builder.write_at("compat_id[1]", 7u32);
        iftx_builder.write_at("compat_id[2]", 8u32);
        iftx_builder.write_at("compat_id[3]", 9u32);
        iftx_builder.write_at("id_delta", Int24::new(1));

        let font = test_font_for_patching_with_loca_mod(
            |_| {},
            HashMap::from([
                (Tag::new(b"IFT "), ift_builder.as_slice()),
                (Tag::new(b"IFTX"), iftx_builder.as_slice()),
            ]),
        );

        let font = FontRef::new(font.as_slice()).unwrap();

        let s = SubsetDefinition::codepoints([5].into_iter().collect());
        let g = PatchGroup::select_next_patches(font, &s).unwrap();

        let patch1 =
            assemble_glyph_keyed_patch(glyph_keyed_patch_header(), glyf_u16_glyph_patches());

        let mut patch2 = glyf_u16_glyph_patches();
        patch2.write_at("gid_13", 14u16);
        let patch2 = assemble_glyph_keyed_patch(glyph_keyed_patch_header(), patch2);

        let mut patch_data = HashMap::from([
            (
                "foo/04".to_string(),
                UriStatus::Pending(patch1.as_slice().to_vec()),
            ),
            (
                "foo/08".to_string(),
                UriStatus::Pending(patch2.as_slice().to_vec()),
            ),
        ]);

        let new_font = g.apply_next_patches(&mut patch_data).unwrap();
        let new_font = FontRef::new(&new_font).unwrap();

        let new_glyf: &[u8] = new_font.table_data(Tag::new(b"glyf")).unwrap().as_bytes();
        assert_eq!(
            &[
                1, 2, 3, 4, 5, 0, // gid 0
                6, 7, 8, 0, // gid 1
                b'a', b'b', b'c', 0, // gid2
                b'd', b'e', b'f', b'g', // gid 7
                b'h', b'i', b'j', b'k', b'l', 0, // gid 8 + 9
                b'm', b'n', // gid 13
                b'm', b'n', // gid 14
            ],
            new_glyf
        );

        assert_eq!(
            patch_data,
            HashMap::from([
                ("foo/04".to_string(), UriStatus::Applied,),
                ("foo/08".to_string(), UriStatus::Applied,),
            ])
        );
    }

    // TODO(garretrieger): test that previously applied patches are ignored.
}