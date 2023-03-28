use super::{
    metrics::{GlyphMetrics, Metrics},
    strings::{LocalizedStrings, StringId, StringIds},
};
use crate::{NormalizedCoord, NormalizedCoords, Size};

/// Interface for types that can provide font metadata.
pub trait MetadataProvider<'a>: raw::TableProvider<'a> + Sized {
    /// Returns an iterator over the available informational string identifiers.
    fn string_ids(&self) -> StringIds<'a> {
        StringIds::new(self)
    }

    /// Returns an iterator over the collection of localized strings for the given informational
    /// string identifier.
    fn localized_strings(&self, id: StringId) -> LocalizedStrings<'a> {
        LocalizedStrings::new(self, id)
    }

    /// Returns the global font metrics for the specified size and normalized variation
    /// coordinates.
    fn metrics(&self, size: Size, coords: NormalizedCoords<'a>) -> Metrics {
        Metrics::new(self, size, coords)
    }

    /// Returns the glyph specific metrics for the specified size and normalized variation
    /// coordinates.
    fn glyph_metrics(&self, size: Size, coords: NormalizedCoords<'a>) -> GlyphMetrics<'a> {
        GlyphMetrics::new(self, size, coords)
    }
}

/// Blanket implementation of `MetadataProvider` for any type that implements
/// `TableProvider`.
impl<'a, T> MetadataProvider<'a> for T where T: raw::TableProvider<'a> {}