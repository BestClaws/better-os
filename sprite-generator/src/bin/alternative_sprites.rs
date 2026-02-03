use std::fs;

use gfx::colors::Color;
use gfx::luma4::Luma4Rasterizer;
use gfx::primitives::{CornerRadius, FillStyle, Gradient, GradientStop, Rectangle, StrokeColor, StrokeStyle};
use gfx::rgb565::Rgb565Rasterizer;
use sprite_generator::{save_luma4_as_bmp, save_rgb565_as_bmp};
use zeno::{Bounds, Point, Stroke};
use zeno::Cap;

fn main() {
    let output_dir = "output/alternative";
    fs::create_dir_all(output_dir).expect("Failed to create alternative output directory");

    let configs = sprite_configs();
    let total = configs.len();

    for config in configs {
        render_sprite(output_dir, &config);
    }

    println!("Generated {} sprite variants (x2 formats)", total);
}

fn render_sprite(output_dir: &str, config: &SpriteConfig) {
    let area = config.area_bounds();
    let clip = config.clip_bounds();
    let corner_radii = config.corner_radii();
    let fill = config.fill_style();
    let edges = config.edges();

    let rectangle = Rectangle {
        area,
        fill,
        edges,
        clip,
        corner_radii,
    };

    let pixel_count = config.pixel_count();

    let mut luma_buffer = vec![0u8; (pixel_count + 1) / 2];
    let mut luma_rasterizer = Luma4Rasterizer::new(&mut luma_buffer, config.width, config.height);
    rectangle.draw(&mut luma_rasterizer);
    let luma_path = format!("{}/luma4_{}.bmp", output_dir, config.name);
    save_luma4_as_bmp(&luma_buffer, config.width, config.height, &luma_path);

    let mut rgb565_buffer = vec![0u8; pixel_count * 2];
    let mut rgb565_rasterizer = Rgb565Rasterizer::new(&mut rgb565_buffer, config.width, config.height);
    rectangle.draw(&mut rgb565_rasterizer);
    let rgb565_path = format!("{}/rgb565_{}.bmp", output_dir, config.name);
    save_rgb565_as_bmp(&rgb565_buffer, config.width, config.height, &rgb565_path);

    println!("Generated: {}", config.name);
}

#[derive(Clone)]
struct SpriteConfig {
    name: String,
    width: u16,
    height: u16,
    margins: Margins,
    fill_spec: FillSpec,
    stroke_spec: Option<StrokeSpec>,
    corner_radius: [u16; 4],
}

impl SpriteConfig {
    fn pixel_count(&self) -> usize {
        self.width as usize * self.height as usize
    }

    fn area_bounds(&self) -> Bounds {
        Bounds::new(
            Point::new(self.margins.left as f32, self.margins.top as f32),
            Point::new(
                (self.width - self.margins.right) as f32,
                (self.height - self.margins.bottom) as f32,
            ),
        )
    }

    fn clip_bounds(&self) -> Bounds {
        Bounds::new(
            Point::new(0.0, 0.0),
            Point::new(self.width as f32, self.height as f32),
        )
    }

    fn corner_radii(&self) -> [CornerRadius; 4] {
        [
            CornerRadius::new(self.corner_radius[0] as f32, self.corner_radius[0] as f32),
            CornerRadius::new(self.corner_radius[1] as f32, self.corner_radius[1] as f32),
            CornerRadius::new(self.corner_radius[2] as f32, self.corner_radius[2] as f32),
            CornerRadius::new(self.corner_radius[3] as f32, self.corner_radius[3] as f32),
        ]
    }

    fn fill_style(&self) -> FillStyle<3> {
        self.fill_spec.to_fill()
    }

    fn edges(&self) -> [Option<StrokeStyle<'static, 2>>; 4] {
        self
            .stroke_spec
            .as_ref()
            .map(|spec| spec.to_edges())
            .unwrap_or([None, None, None, None])
    }
}

#[derive(Clone, Copy)]
struct Margins {
    left: u16,
    top: u16,
    right: u16,
    bottom: u16,
}

impl Margins {
    const fn new(left: u16, top: u16, right: u16, bottom: u16) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }
}

#[derive(Clone, Copy)]
enum FillSpec {
    Solid(Color),
    Gradient {
        orientation: Orientation,
        stops: [(Color, u8); 3],
    },
}

impl FillSpec {
    fn to_fill(&self) -> FillStyle<3> {
        match self {
            FillSpec::Solid(color) => FillStyle::Solid(*color),
            FillSpec::Gradient { orientation, stops } => {
                FillStyle::Gradient(orientation.to_fill_gradient(*stops))
            }
        }
    }
}

#[derive(Clone, Copy)]
enum StrokeStyleSpec {
    Solid(Color),
    Gradient {
        orientation: Orientation,
        stops: [(Color, u8); 2],
    },
}

impl StrokeStyleSpec {
    fn to_color(&self) -> StrokeColor<2> {
        match self {
            StrokeStyleSpec::Solid(color) => StrokeColor::Solid(*color),
            StrokeStyleSpec::Gradient { orientation, stops } => {
                StrokeColor::Gradient(orientation.to_stroke_gradient(*stops))
            }
        }
    }
}

#[derive(Clone, Copy)]
enum CapStyle {
    Flat,
}

#[derive(Clone, Copy)]
struct StrokeSpec {
    width: u16,
    cap: CapStyle,
    style: StrokeStyleSpec,
}

impl StrokeSpec {
    fn to_style(&self) -> StrokeStyle<'static, 2> {
        let mut stroke = Stroke::new(self.width as f32);
        match self.cap {
            CapStyle::Flat => {
                stroke.cap(Cap::Butt);
            }
        }

        StrokeStyle {
            color: self.style.to_color(),
            stroke,
        }
    }

    fn to_edges(&self) -> [Option<StrokeStyle<'static, 2>>; 4] {
        let style = self.to_style();
        [
            Some(style.clone()),
            Some(style.clone()),
            Some(style.clone()),
            Some(style),
        ]
    }
}

#[derive(Clone, Copy)]
enum Orientation {
    Horizontal,
    Vertical,
}

impl Orientation {
    fn to_fill_gradient(&self, stops: [(Color, u8); 3]) -> Gradient<3> {
        match self {
            Orientation::Horizontal => Gradient::Horizontal(GradientStop(stops)),
            Orientation::Vertical => Gradient::Vertical(GradientStop(stops)),
        }
    }

    fn to_stroke_gradient(&self, stops: [(Color, u8); 2]) -> Gradient<2> {
        match self {
            Orientation::Horizontal => Gradient::Horizontal(GradientStop(stops)),
            Orientation::Vertical => Gradient::Vertical(GradientStop(stops)),
        }
    }
}

#[derive(Clone, Copy)]
enum FillType {
    None,
    Solid,
    Gradient(Orientation),
}

#[derive(Clone, Copy)]
enum StrokeType {
    None,
    Solid,
    Gradient(Orientation),
}

fn sprite_configs() -> Vec<SpriteConfig> {
    const WIDTH: u16 = 128;
    const HEIGHT: u16 = 128;
    const CORNER_RADIUS: [u16; 4] = [4, 4, 4, 4];
    const STROKE_WIDTH: u16 = 2;
    const MARGIN: u16 = 32;

    let color_variants: [(&str, Color); 3] = [
        ("midnight", Color::rgba(0x2C, 0x3E, 0x70, 0xFF)),
        ("sunset", Color::rgba(0xFF, 0x88, 0x44, 0xFF)),
        ("verdant", Color::rgba(0x3A, 0xC4, 0x80, 0xFF)),
    ];
    let alpha_variants: [(&str, u8); 3] = [
        ("opaque", 0xFF),
        ("soft", 0xC0),
        ("light", 0x88),
    ];
    let fill_variants: [(&str, FillType); 4] = [
        ("none", FillType::None),
        ("solid", FillType::Solid),
        ("grad_h", FillType::Gradient(Orientation::Horizontal)),
        ("grad_v", FillType::Gradient(Orientation::Vertical)),
    ];
    let stroke_variants: [(&str, StrokeType); 4] = [
        ("none", StrokeType::None),
        ("solid", StrokeType::Solid),
        ("grad_h", StrokeType::Gradient(Orientation::Horizontal)),
        ("grad_v", StrokeType::Gradient(Orientation::Vertical)),
    ];

    let mut configs = Vec::new();

    for (color_name, base_color) in color_variants.iter() {
        for (alpha_name, alpha) in alpha_variants.iter() {
            for (fill_name, fill_kind) in fill_variants.iter() {
                for (stroke_name, stroke_kind) in stroke_variants.iter() {
                    let sprite_name = format!(
                        "{}_{}_fill-{}_stroke-{}",
                        color_name, alpha_name, fill_name, stroke_name
                    );
                    let fill_spec = build_fill_spec(*fill_kind, *base_color, *alpha);
                    let stroke_spec = build_stroke_spec(*stroke_kind, *base_color, *alpha, STROKE_WIDTH);

                    configs.push(SpriteConfig {
                        name: sprite_name,
                        width: WIDTH,
                        height: HEIGHT,
                        margins: Margins::new(MARGIN, MARGIN, MARGIN, MARGIN),
                        fill_spec,
                        stroke_spec,
                        corner_radius: CORNER_RADIUS,
                    });
                }
            }
        }
    }

    configs
}

fn build_fill_spec(fill_type: FillType, base_color: Color, alpha: u8) -> FillSpec {
    match fill_type {
        FillType::None => FillSpec::Solid(Color::rgba(0, 0, 0, 0)),
        FillType::Solid => FillSpec::Solid(apply_alpha(base_color, alpha)),
        FillType::Gradient(orientation) => FillSpec::Gradient {
            orientation,
            stops: fill_gradient_stops(base_color, alpha),
        },
    }
}

fn build_stroke_spec(
    stroke_type: StrokeType,
    base_color: Color,
    alpha: u8,
    width: u16,
) -> Option<StrokeSpec> {
    match stroke_type {
        StrokeType::None => None,
        StrokeType::Solid => Some(StrokeSpec {
            width,
            cap: CapStyle::Flat,
            style: StrokeStyleSpec::Solid(apply_alpha(base_color, alpha)),
        }),
        StrokeType::Gradient(orientation) => Some(StrokeSpec {
            width,
            cap: CapStyle::Flat,
            style: StrokeStyleSpec::Gradient {
                orientation,
                stops: stroke_gradient_stops(base_color, alpha),
            },
        }),
    }
}

fn fill_gradient_stops(base_color: Color, alpha: u8) -> [(Color, u8); 3] {
    let deltas = [-40, 0, 40];
    let positions = [0x00, 0x80, 0xFF];
    let mut stops = [(Color::rgba(0, 0, 0, 0), 0u8); 3];

    for ((stop, delta), position) in stops.iter_mut().zip(deltas.iter()).zip(positions.iter()) {
        *stop = (tint_color(base_color, *delta, alpha), *position);
    }

    stops
}

fn stroke_gradient_stops(base_color: Color, alpha: u8) -> [(Color, u8); 2] {
    let deltas = [-32, 32];
    let positions = [0x00, 0xFF];
    let mut stops = [(Color::rgba(0, 0, 0, 0), 0u8); 2];

    for ((stop, delta), position) in stops.iter_mut().zip(deltas.iter()).zip(positions.iter()) {
        *stop = (tint_color(base_color, *delta, alpha), *position);
    }

    stops
}

fn tint_color(base_color: Color, delta: i16, alpha: u8) -> Color {
    let adjust = |channel: u8| -> u8 {
        let value = channel as i16 + delta;
        value.clamp(0, 255) as u8
    };

    Color::rgba(
        adjust(base_color.r()),
        adjust(base_color.g()),
        adjust(base_color.b()),
        alpha,
    )
}

fn apply_alpha(color: Color, alpha: u8) -> Color {
    Color::rgba(color.r(), color.g(), color.b(), alpha)
}
