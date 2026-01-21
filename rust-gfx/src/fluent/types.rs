//! Core helper types shared by the fluent builders.

extern crate alloc;

use alloc::borrow::Cow;
use alloc::string::String;

use crate::types::{Area, Point};

/// Angular value stored in whole degrees.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Angle(i32);

impl Angle {
    /// Construct an angle from degrees.
    pub fn from_degrees(value: i32) -> Self {
        Self(value)
    }

    /// Inspect the raw degree value.
    pub fn as_degrees(self) -> i32 {
        self.0
    }
}

/// Pair of start/end angles used by arc primitives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Angles {
    start: Angle,
    end: Angle,
}

impl Angles {
    pub fn new(start: Angle, end: Angle) -> Self {
        Self { start, end }
    }

    pub fn start(self) -> Angle {
        self.start
    }

    pub fn end(self) -> Angle {
        self.end
    }
}

/// Radius descriptors used by the fluent primitives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Radius {
    /// Uniform radius for all corners or circular primitives.
    Uniform(i32),
    /// Per-corner radius values (top-left, top-right, bottom-right, bottom-left).
    Corners {
        top_left: i32,
        top_right: i32,
        bottom_right: i32,
        bottom_left: i32,
    },
    /// Inner/outer radius pair for arc primitives.
    Ring { inner: i32, outer: i32 },
}

impl Radius {
    pub fn uniform(value: i32) -> Self {
        Self::Uniform(value)
    }

    pub fn corners(top_left: i32, top_right: i32, bottom_right: i32, bottom_left: i32) -> Self {
        Self::Corners {
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        }
    }

    pub fn ring(inner: i32, outer: i32) -> Self {
        Self::Ring { inner, outer }
    }

    pub fn inner(self) -> Option<i32> {
        match self {
            Radius::Ring { inner, .. } => Some(inner),
            _ => None,
        }
    }

    pub fn outer(self) -> Option<i32> {
        match self {
            Radius::Ring { outer, .. } => Some(outer),
            Radius::Uniform(value) => Some(value),
            Radius::Corners { .. } => None,
        }
    }
}

/// Helper supporting 2- or 3-point vertex lists.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertices {
    points: [Point; 3],
    count: usize,
}

impl Vertices {
    pub fn new(p1: Point, p2: Point) -> Self {
        Self {
            points: [p1, p2, p2],
            count: 2,
        }
    }

    pub fn new3(a: Point, b: Point, c: Point) -> Self {
        Self {
            points: [a, b, c],
            count: 3,
        }
    }

    pub fn as_slice(&self) -> &[Point] {
        &self.points[..self.count]
    }
}

/// Line cap configuration used by the fluent line builder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineCap {
    Butt,
    Round,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineCaps {
    pub start: LineCap,
    pub end: LineCap,
}

impl LineCaps {
    pub fn butt() -> Self {
        Self {
            start: LineCap::Butt,
            end: LineCap::Butt,
        }
    }

    pub fn round() -> Self {
        Self {
            start: LineCap::Round,
            end: LineCap::Round,
        }
    }

    pub fn with(start: LineCap, end: LineCap) -> Self {
        Self { start, end }
    }
}

impl Default for LineCaps {
    fn default() -> Self {
        Self::butt()
    }
}

/// Unit value for dash pattern entries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DashUnit {
    pixels: i32,
}

impl DashUnit {
    pub fn pixels(pixels: i32) -> Self {
        Self {
            pixels: pixels.max(0),
        }
    }

    pub fn raw(self) -> i32 {
        self.pixels
    }
}

const MAX_DASH_UNITS: usize = 8;

/// Fixed-capacity dash pattern container matching embedded expectations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DashPattern {
    units: [DashUnit; MAX_DASH_UNITS],
    count: usize,
}

impl DashPattern {
    pub fn new(units: &[DashUnit]) -> Self {
        let mut pattern = Self {
            units: [DashUnit::pixels(0); MAX_DASH_UNITS],
            count: 0,
        };
        for &unit in units.iter().take(MAX_DASH_UNITS) {
            pattern.units[pattern.count] = unit;
            pattern.count += 1;
        }
        pattern
    }

    pub fn iter(&self) -> impl Iterator<Item = DashUnit> + '_ {
        self.units[..self.count].iter().copied()
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

/// Label alignment options.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelAlignment {
    Start,
    Center,
    End,
}

impl LabelAlignment {
    pub fn start() -> Self {
        Self::Start
    }

    pub fn centered() -> Self {
        Self::Center
    }

    pub fn end() -> Self {
        Self::End
    }
}

impl Default for LabelAlignment {
    fn default() -> Self {
        Self::Start
    }
}

/// Label decoration options for the fluent interface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelDecor {
    None,
    Underline,
    Strikethrough,
}

impl LabelDecor {
    pub fn none() -> Self {
        Self::None
    }

    pub fn underline() -> Self {
        Self::Underline
    }

    pub fn strikethrough() -> Self {
        Self::Strikethrough
    }
}

impl Default for LabelDecor {
    fn default() -> Self {
        Self::None
    }
}

/// Letter/line spacing configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LabelSpacing {
    letter: i32,
    line: i32,
}

impl LabelSpacing {
    pub fn new(letter: i32, line: i32) -> Self {
        Self { letter, line }
    }

    pub fn letter(self) -> i32 {
        self.letter
    }

    pub fn line(self) -> i32 {
        self.line
    }
}

impl Default for LabelSpacing {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

/// Wrapper for opacity selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LabelOpacity(pub u8);

impl LabelOpacity {
    pub fn new(opa: u8) -> Self {
        Self(opa)
    }

    pub fn value(self) -> u8 {
        self.0
    }
}

impl Default for LabelOpacity {
    fn default() -> Self {
        Self(crate::types::OPA_COVER)
    }
}

/// Handle for selecting built-in fonts by name/size.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontHandle<'a> {
    pub name: Cow<'a, str>,
    pub size_px: u8,
}

impl<'a> FontHandle<'a> {
    pub fn named<N>(name: N, size_px: u8) -> Self
    where
        N: Into<Cow<'a, str>>,
    {
        Self {
            name: name.into(),
            size_px,
        }
    }
}

/// Prepared label content selection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LabelContentPlan<'a> {
    Text {
        font: FontHandle<'a>,
        text: Cow<'a, str>,
    },
}

impl<'a> LabelContentPlan<'a> {
    pub fn text<S>(font: FontHandle<'a>, text: S) -> Self
    where
        S: Into<Cow<'a, str>>,
    {
        Self::Text {
            font,
            text: text.into(),
        }
    }

    pub fn into_owned(self) -> LabelContent {
        match self {
            LabelContentPlan::Text { font, text } => LabelContent::Text {
                font: font.into_owned(),
                text: text.into_owned(),
            },
        }
    }
}

/// Owned variant used internally after `finish()`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LabelContent {
    Text {
        font: FontHandle<'static>,
        text: String,
    },
}

impl<'a> FontHandle<'a> {
    fn into_owned(self) -> FontHandle<'static> {
        FontHandle {
            name: Cow::Owned(self.name.into_owned()),
            size_px: self.size_px,
        }
    }
}

/// Wrapper for configured winding rule in vector primitives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindingPlan {
    NonZero,
    EvenOdd,
}

impl WindingPlan {
    pub fn non_zero() -> Self {
        Self::NonZero
    }

    pub fn even_odd() -> Self {
        Self::EvenOdd
    }
}

impl Default for WindingPlan {
    fn default() -> Self {
        Self::NonZero
    }
}

/// Path plan currently wraps SVG data. Parsing occurs during `finish()`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathPlan<'a> {
    pub svg: Cow<'a, str>,
}

impl<'a> PathPlan<'a> {
    pub fn from_svg<S>(data: S) -> Self
    where
        S: Into<Cow<'a, str>>,
    {
        Self { svg: data.into() }
    }

    pub fn into_owned(self) -> PathPlan<'static> {
        PathPlan {
            svg: Cow::Owned(self.svg.into_owned()),
        }
    }
}

/// Extension helpers aligned with the fluent examples for constructing areas.
impl Area {
    pub fn from_corners(p1: Point, p2: Point) -> Self {
        Self::new(p1.x, p1.y, p2.x, p2.y)
    }
}
