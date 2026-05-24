use egui::{Rect, Vec2, pos2};

pub fn calculate_cover_uv(tex_size: Vec2, target_size: Vec2) -> Rect {
    let tex_aspect = tex_size.x / tex_size.y;
    let target_aspect = target_size.x / target_size.y;

    if tex_aspect > target_aspect {
        // Image is wider than target. Crop left/right.
        let scale = target_size.y / tex_size.y;
        let drawn_width = tex_size.x * scale;
        let extra_width = drawn_width - target_size.x;
        let crop_u = (extra_width / 2.0) / drawn_width;
        Rect::from_min_max(pos2(crop_u, 0.0), pos2(1.0 - crop_u, 1.0))
    } else {
        // Image is taller than target. Crop top/bottom.
        let scale = target_size.x / tex_size.x;
        let drawn_height = tex_size.y * scale;
        let extra_height = drawn_height - target_size.y;
        let crop_v = (extra_height / 2.0) / drawn_height;
        Rect::from_min_max(pos2(0.0, crop_v), pos2(1.0, 1.0 - crop_v))
    }
}
