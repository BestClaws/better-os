use std::fs;
use gfx::colors::Color;
use gfx::luma4::Luma4Rasterizer;
use gfx::rgb565::Rgb565Rasterizer;
use gfx::primitives::{CornerRadius, FillStyle, Gradient, GradientStop, Rectangle, StrokeColor, StrokeStyle};
use zeno::{Bounds, Point, Stroke};
use sprite_generator::{save_luma4_as_bmp, save_rgb565_as_bmp};

fn main() {
    println!("Generating rectangle sprite BMPs with permutations...");
    
    // Create output directory
    fs::create_dir_all("output").expect("Failed to create output directory");
    
    // Generate all permutation variants for both pixel formats
    generate_rectangle_permutations();
    
    println!("\nAll sprites generated successfully!");
}

fn generate_rectangle_permutations() {
    let width = 20u16;
    let height = 20u16;
    
    // Rectangle bounds: 16x16 centered in 20x20 sprite
    let area = Bounds::new(Point::new(2.0, 2.0), Point::new(18.0, 18.0));
    let clip = Bounds::new(Point::new(0.0, 0.0), Point::new(width as f32, height as f32));
    
    let mut test_num = 0;
    
    // Permutation loops: 3 fills × 4 corners × 6 strokes = 72 variants
    for fill_idx in 0..3 {
        let (fill, fill_name) = match fill_idx {
            0 => {
                // Solid gray fill
                (FillStyle::Solid(Color::rgba(150, 150, 150, 255)), "solid")
            }
            1 => {
                // Vertical gradient (dark to light)
                (FillStyle::Gradient(Gradient::Vertical(GradientStop([
                    (Color::rgba(40, 40, 40, 255), 0),
                    (Color::rgba(180, 180, 180, 255), 128),
                    (Color::rgba(255, 255, 255, 255), 255),
                ]))), "vgrad")
            }
            2 => {
                // Horizontal gradient (dark to light)
                (FillStyle::Gradient(Gradient::Horizontal(GradientStop([
                    (Color::rgba(40, 40, 40, 255), 0),
                    (Color::rgba(180, 180, 180, 255), 128),
                    (Color::rgba(255, 255, 255, 255), 255),
                ]))), "hgrad")
            }
            _ => unreachable!(),
        };
        
        for corner_idx in 0..4 {
            let (corner_radii, corner_name) = match corner_idx {
                0 => ([CornerRadius::new(0.0, 0.0); 4], "sharp"),
                1 => ([CornerRadius::new(4.0, 4.0); 4], "r4"),
                2 => ([CornerRadius::new(6.0, 6.0); 4], "r6"),
                3 => (
                    [
                        CornerRadius::new(0.0, 0.0),
                        CornerRadius::new(4.0, 4.0),
                        CornerRadius::new(6.0, 6.0),
                        CornerRadius::new(8.0, 8.0),
                    ],
                    "multi"
                ),
                _ => unreachable!(),
            };
            
            for stroke_idx in 0..9 {
                let (edges, stroke_name) = match stroke_idx {
                    0 => ([None, None, None, None], "nostroke"),
                    1 => (
                        [
                            Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(1.0) }),
                            Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(1.0) }),
                            Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(1.0) }),
                            Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(1.0) }),
                        ],
                        "s1"
                    ),
                    2 => (
                        [
                            Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(2.0) }),
                            Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(2.0) }),
                            Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(2.0) }),
                            Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(2.0) }),
                        ],
                        "s2"
                    ),
                    3 => (
                        [
                            Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(3.0) }),
                            Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(3.0) }),
                            Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(3.0) }),
                            Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(3.0) }),
                        ],
                        "s3"
                    ),
                    4 => (
                        [
                            Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(1.0) }),
                            Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(2.0) }),
                            Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(3.0) }),
                            Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(4.0) }),
                        ],
                        "asym_w"
                    ),
                    5 => (
                        [
                            Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 100, 100, 255)), stroke: Stroke::new(2.0) }),
                            Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(100, 255, 100, 255)), stroke: Stroke::new(2.0) }),
                            Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(100, 100, 255, 255)), stroke: Stroke::new(2.0) }),
                            Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 100, 255)), stroke: Stroke::new(2.0) }),
                        ],
                        "asym_c"
                    ),
                    6 => (
                        [
                            Some(StrokeStyle { 
                                color: StrokeColor::<2>::Gradient(Gradient::Horizontal(GradientStop([
                                    (Color::rgba(255, 100, 100, 255), 0),
                                    (Color::rgba(100, 100, 255, 255), 255),
                                ]))), 
                                stroke: Stroke::new(2.0) 
                            }),
                            Some(StrokeStyle { 
                                color: StrokeColor::<2>::Gradient(Gradient::Horizontal(GradientStop([
                                    (Color::rgba(255, 100, 100, 255), 0),
                                    (Color::rgba(100, 100, 255, 255), 255),
                                ]))), 
                                stroke: Stroke::new(2.0) 
                            }),
                            Some(StrokeStyle { 
                                color: StrokeColor::<2>::Gradient(Gradient::Horizontal(GradientStop([
                                    (Color::rgba(255, 100, 100, 255), 0),
                                    (Color::rgba(100, 100, 255, 255), 255),
                                ]))), 
                                stroke: Stroke::new(2.0) 
                            }),
                            Some(StrokeStyle { 
                                color: StrokeColor::<2>::Gradient(Gradient::Horizontal(GradientStop([
                                    (Color::rgba(255, 100, 100, 255), 0),
                                    (Color::rgba(100, 100, 255, 255), 255),
                                ]))), 
                                stroke: Stroke::new(2.0) 
                            }),
                        ],
                        "grad_h"
                    ),
                    7 => (
                        [
                            Some(StrokeStyle { 
                                color: StrokeColor::<2>::Gradient(Gradient::Vertical(GradientStop([
                                    (Color::rgba(100, 255, 100, 255), 0),
                                    (Color::rgba(255, 255, 100, 255), 255),
                                ]))), 
                                stroke: Stroke::new(2.0) 
                            }),
                            Some(StrokeStyle { 
                                color: StrokeColor::<2>::Gradient(Gradient::Vertical(GradientStop([
                                    (Color::rgba(100, 255, 100, 255), 0),
                                    (Color::rgba(255, 255, 100, 255), 255),
                                ]))), 
                                stroke: Stroke::new(2.0) 
                            }),
                            Some(StrokeStyle { 
                                color: StrokeColor::<2>::Gradient(Gradient::Vertical(GradientStop([
                                    (Color::rgba(100, 255, 100, 255), 0),
                                    (Color::rgba(255, 255, 100, 255), 255),
                                ]))), 
                                stroke: Stroke::new(2.0) 
                            }),
                            Some(StrokeStyle { 
                                color: StrokeColor::<2>::Gradient(Gradient::Vertical(GradientStop([
                                    (Color::rgba(100, 255, 100, 255), 0),
                                    (Color::rgba(255, 255, 100, 255), 255),
                                ]))), 
                                stroke: Stroke::new(2.0) 
                            }),
                        ],
                        "grad_v"
                    ),
                    8 => (
                        [
                            Some(StrokeStyle { 
                                color: StrokeColor::<2>::Gradient(Gradient::Horizontal(GradientStop([
                                    (Color::rgba(255, 80, 80, 255), 0),
                                    (Color::rgba(80, 80, 255, 255), 255),
                                ]))), 
                                stroke: Stroke::new(2.0) 
                            }),
                            Some(StrokeStyle { 
                                color: StrokeColor::<2>::Gradient(Gradient::Vertical(GradientStop([
                                    (Color::rgba(80, 255, 80, 255), 0),
                                    (Color::rgba(255, 255, 80, 255), 255),
                                ]))), 
                                stroke: Stroke::new(2.0) 
                            }),
                            Some(StrokeStyle { 
                                color: StrokeColor::<2>::Gradient(Gradient::Horizontal(GradientStop([
                                    (Color::rgba(255, 140, 200, 255), 0),
                                    (Color::rgba(140, 200, 255, 255), 255),
                                ]))), 
                                stroke: Stroke::new(2.0) 
                            }),
                            Some(StrokeStyle { 
                                color: StrokeColor::<2>::Gradient(Gradient::Vertical(GradientStop([
                                    (Color::rgba(200, 140, 255, 255), 0),
                                    (Color::rgba(255, 200, 140, 255), 255),
                                ]))), 
                                stroke: Stroke::new(2.0) 
                            }),
                        ],
                        "grad_mix"
                    ),
                    _ => unreachable!(),
                };
                
                test_num += 1;
                let filename = format!("rect_{:03}_{}_{}_{}", test_num, fill_name, corner_name, stroke_name);
                
                // Recreate fill since FillStyle doesn't implement Clone
                let fill = match fill_idx {
                    0 => FillStyle::Solid(Color::rgba(100, 180, 220, 255)),  // Cyan-blue
                    1 => FillStyle::Gradient(Gradient::Vertical(GradientStop([
                        (Color::rgba(220, 60, 100, 255), 0),     // Pink-red
                        (Color::rgba(120, 180, 240, 255), 128),  // Sky blue
                        (Color::rgba(100, 255, 150, 255), 255),  // Mint green
                    ]))),
                    2 => FillStyle::Gradient(Gradient::Horizontal(GradientStop([
                        (Color::rgba(240, 180, 60, 255), 0),     // Orange-yellow
                        (Color::rgba(160, 100, 220, 255), 128),  // Purple
                        (Color::rgba(80, 220, 200, 255), 255),   // Turquoise
                    ]))),
                    _ => unreachable!(),
                };
                
                // Recreate edges since StrokeStyle doesn't implement Copy
                let edges: [Option<StrokeStyle<'_, 2>>; 4] = match stroke_idx {
                    0 => [None, None, None, None],
                    1 => [
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(1.0) }),
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(1.0) }),
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(1.0) }),
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(1.0) }),
                    ],
                    2 => [
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(2.0) }),
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(2.0) }),
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(2.0) }),
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(2.0) }),
                    ],
                    3 => [
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(3.0) }),
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(3.0) }),
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(3.0) }),
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(3.0) }),
                    ],
                    4 => [
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(1.0) }),
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(2.0) }),
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(3.0) }),
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(4.0) }),
                    ],
                    5 => [
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 100, 100, 255)), stroke: Stroke::new(2.0) }),
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(100, 255, 100, 255)), stroke: Stroke::new(2.0) }),
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(100, 100, 255, 255)), stroke: Stroke::new(2.0) }),
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 100, 255)), stroke: Stroke::new(2.0) }),
                    ],
                    6 => [
                        Some(StrokeStyle { 
                            color: StrokeColor::<2>::Gradient(Gradient::Horizontal(GradientStop([
                                (Color::rgba(255, 100, 100, 255), 0),
                                (Color::rgba(100, 100, 255, 255), 255),
                            ]))), 
                            stroke: Stroke::new(2.0) 
                        }),
                        Some(StrokeStyle { 
                            color: StrokeColor::<2>::Gradient(Gradient::Horizontal(GradientStop([
                                (Color::rgba(255, 100, 100, 255), 0),
                                (Color::rgba(100, 100, 255, 255), 255),
                            ]))), 
                            stroke: Stroke::new(2.0) 
                        }),
                        Some(StrokeStyle { 
                            color: StrokeColor::<2>::Gradient(Gradient::Horizontal(GradientStop([
                                (Color::rgba(255, 100, 100, 255), 0),
                                (Color::rgba(100, 100, 255, 255), 255),
                            ]))), 
                            stroke: Stroke::new(2.0) 
                        }),
                        Some(StrokeStyle { 
                            color: StrokeColor::<2>::Gradient(Gradient::Horizontal(GradientStop([
                                (Color::rgba(255, 100, 100, 255), 0),
                                (Color::rgba(100, 100, 255, 255), 255),
                            ]))), 
                            stroke: Stroke::new(2.0) 
                        }),
                    ],
                    7 => [
                        Some(StrokeStyle { 
                            color: StrokeColor::<2>::Gradient(Gradient::Vertical(GradientStop([
                                (Color::rgba(100, 255, 100, 255), 0),
                                (Color::rgba(255, 255, 100, 255), 255),
                            ]))), 
                            stroke: Stroke::new(2.0) 
                        }),
                        Some(StrokeStyle { 
                            color: StrokeColor::<2>::Gradient(Gradient::Vertical(GradientStop([
                                (Color::rgba(100, 255, 100, 255), 0),
                                (Color::rgba(255, 255, 100, 255), 255),
                            ]))), 
                            stroke: Stroke::new(2.0) 
                        }),
                        Some(StrokeStyle { 
                            color: StrokeColor::<2>::Gradient(Gradient::Vertical(GradientStop([
                                (Color::rgba(100, 255, 100, 255), 0),
                                (Color::rgba(255, 255, 100, 255), 255),
                            ]))), 
                            stroke: Stroke::new(2.0) 
                        }),
                        Some(StrokeStyle { 
                            color: StrokeColor::<2>::Gradient(Gradient::Vertical(GradientStop([
                                (Color::rgba(100, 255, 100, 255), 0),
                                (Color::rgba(255, 255, 100, 255), 255),
                            ]))), 
                            stroke: Stroke::new(2.0) 
                        }),
                    ],
                    8 => [
                        Some(StrokeStyle { 
                            color: StrokeColor::<2>::Gradient(Gradient::Horizontal(GradientStop([
                                (Color::rgba(255, 80, 80, 255), 0),
                                (Color::rgba(80, 80, 255, 255), 255),
                            ]))), 
                            stroke: Stroke::new(2.0) 
                        }),
                        Some(StrokeStyle { 
                            color: StrokeColor::<2>::Gradient(Gradient::Vertical(GradientStop([
                                (Color::rgba(80, 255, 80, 255), 0),
                                (Color::rgba(255, 255, 80, 255), 255),
                            ]))), 
                            stroke: Stroke::new(2.0) 
                        }),
                        Some(StrokeStyle { 
                            color: StrokeColor::<2>::Gradient(Gradient::Horizontal(GradientStop([
                                (Color::rgba(255, 140, 200, 255), 0),
                                (Color::rgba(140, 200, 255, 255), 255),
                            ]))), 
                            stroke: Stroke::new(2.0) 
                        }),
                        Some(StrokeStyle { 
                            color: StrokeColor::<2>::Gradient(Gradient::Vertical(GradientStop([
                                (Color::rgba(200, 140, 255, 255), 0),
                                (Color::rgba(255, 200, 140, 255), 255),
                            ]))), 
                            stroke: Stroke::new(2.0) 
                        }),
                    ],
                    _ => unreachable!(),
                };
                
                let rect = Rectangle {
                    area,
                    fill,
                    edges,
                    clip,
                    corner_radii,
                };
                
                // Generate Luma4 version
                {
                    let pixel_count = (width as usize) * (height as usize);
                    let mut buffer = vec![0u8; (pixel_count + 1) / 2];
                    let mut rasterizer = Luma4Rasterizer::new(&mut buffer, width, height);
                    rect.draw(&mut rasterizer);
                    save_luma4_as_bmp(&buffer, width, height, &format!("output/luma4_{}.bmp", filename));
                }
                
                // Generate RGB565 version
                {
                    let pixel_count = (width as usize) * (height as usize);
                    let mut buffer = vec![0u8; pixel_count * 2];
                    let mut rasterizer = Rgb565Rasterizer::new(&mut buffer, width, height);
                    rect.draw(&mut rasterizer);
                    save_rgb565_as_bmp(&buffer, width, height, &format!("output/rgb565_{}.bmp", filename));
                }
                
                println!("Generated: {} (luma4 + rgb565)", filename);
            }
        }
    }
    
    println!("\nTotal variants generated: {} (×2 for both pixel formats = {} files)", test_num, test_num * 2);
}
