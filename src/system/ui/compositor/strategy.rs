use crate::libs::gfx::two_d::Rect;
use crate::system::ui::display::Display;

#[derive(Debug)]
pub enum UpdateStrategy {
    FullScreen,
    Partial(heapless::Vec<Rect, 8>),
}

pub fn determine_update_strategy(display: Option<&Display>, dirty_regions: &heapless::Vec<Rect, 8>) -> UpdateStrategy {
    if dirty_regions.is_empty() { return UpdateStrategy::FullScreen; }
    let full_area = if let Some(service) = display { service.width() * service.height() } else { 0 };
    let mut total_area: u32 = 0;
    for r in dirty_regions.iter() {
        total_area = total_area.saturating_add(r.size.width.saturating_mul(r.size.height));
    }
    if dirty_regions.len() > 6 || (full_area != 0 && total_area * 3 > full_area) {
        UpdateStrategy::FullScreen
    } else {
        let mut regions = heapless::Vec::new();
        for region in dirty_regions.iter() { let _ = regions.push(*region); }
        UpdateStrategy::Partial(regions)
    }
}


