use rust_gfx::color::Rgba8888;
use rust_gfx::masks::{LineMask, LineSide, Mask, MaskResult};
use rust_gfx::primitives::triangle::*;
use rust_gfx::types::*;
/// Debug triangle mask generation to find the ±1-2 pixel differences
use rust_gfx::*;

fn main() {
    // Test the triangle that has 11 pixel differences
    // tri_up_red: p1=(51,30), p2=(25,90), p3=(77,90)
    let p1 = Point::new(51, 30);
    let p2 = Point::new(25, 90);
    let p3 = Point::new(77, 90);

    // Sort like triangle code does
    let mut p = [p1, p2, p3];
    if p[0].y > p[2].y {
        p.swap(0, 2);
    }
    if p[0].y > p[1].y {
        p.swap(0, 1);
    }
    if p[1].y < p[2].y {
        p.swap(1, 2);
    }

    let right = ((p[1].x - p[0].x) * (p[2].y - p[0].y) - (p[1].y - p[0].y) * (p[2].x - p[0].x)) < 0;

    println!("Sorted vertices: {:?}, {:?}, {:?}", p[0], p[1], p[2]);
    println!("Right side: {}", right);

    // Create masks
    let mask_left = LineMask::from_points(
        p[0],
        p[1],
        if right {
            LineSide::Right
        } else {
            LineSide::Left
        },
    );

    let mask_right = LineMask::from_points(
        p[0],
        p[2],
        if right {
            LineSide::Left
        } else {
            LineSide::Right
        },
    );

    let mask_bottom = LineMask::from_points(p[1], p[2], LineSide::Top);

    // Test the failing pixels
    let test_pixels = vec![
        (64, 39), // LVGL=255, Rust=253
        (61, 43), // LVGL=0, Rust=1
        (61, 46), // LVGL=255, Rust=254
        (74, 46), // LVGL=255, Rust=254
        (77, 50), // LVGL=0, Rust=2
    ];

    for (x, y) in test_pixels {
        println!("\nPixel ({}, {}):", x, y);

        // Apply each mask individually
        let mut buf1 = vec![255u8; 1];
        let res1 = mask_left.apply(&mut buf1, x, y, 1);
        println!("  mask_left: {:?} = {}", res1, buf1[0]);

        let mut buf2 = vec![255u8; 1];
        let res2 = mask_right.apply(&mut buf2, x, y, 1);
        println!("  mask_right: {:?} = {}", res2, buf2[0]);

        let mut buf3 = vec![255u8; 1];
        let res3 = mask_bottom.apply(&mut buf3, x, y, 1);
        println!("  mask_bottom: {:?} = {}", res3, buf3[0]);

        // Combine all three
        let mut combined = vec![255u8; 1];
        mask_left.apply(&mut combined, x, y, 1);
        mask_right.apply(&mut combined, x, y, 1);
        mask_bottom.apply(&mut combined, x, y, 1);
        println!("  COMBINED: {}", combined[0]);
    }
}
