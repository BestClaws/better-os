use crate::util::math::primitives::Rect;

#[derive(Debug)]
pub enum UpdateStrategy {
    FullScreen,
    Partial(heapless::Vec<Rect, 8>),
}

pub fn determine_update_strategy(
    full_area: u32,
    dirty_regions: &heapless::Vec<Rect, 8>,
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
        let mut regions = heapless::Vec::new();
        for region in dirty_regions.iter() {
            let _ = regions.push(*region);
        }
        UpdateStrategy::Partial(regions)
    }
}
