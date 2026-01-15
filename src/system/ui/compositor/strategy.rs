use alloc::vec::Vec;

use crate::util::math::primitives::Rect;

#[derive(Debug)]
pub enum UpdateStrategy {
    FullScreen,
    Partial(Vec<Rect>),
}

pub fn determine_update_strategy(
    full_area: u32,
    dirty_regions: &[Rect],
) -> UpdateStrategy {
    if dirty_regions.is_empty() {
        return UpdateStrategy::FullScreen;
    }
    let mut total_area: u32 = 0;
    for r in dirty_regions.iter() {
        total_area = total_area.saturating_add(r.size.width.saturating_mul(r.size.height));
    }
    if dirty_regions.len() > 6 || (full_area != 0 && total_area * 3 > full_area) {
        UpdateStrategy::FullScreen
    } else {
        let mut regions = Vec::with_capacity(dirty_regions.len());
        for region in dirty_regions.iter() {
            regions.push(*region);
        }
        UpdateStrategy::Partial(regions)
    }
}
