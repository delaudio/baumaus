use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Wall {
    pub id: String,
    pub start: Point,
    pub end: Point,
    pub thickness: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OpeningKind {
    Door,
    Window { sill: f64 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Opening {
    pub id: String,
    pub wall_id: String,
    pub offset: f64,
    pub width: f64,
    pub kind: OpeningKind,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub walls: Vec<Wall>,
    pub openings: Vec<Opening>,
}

impl Project {
    #[must_use]
    pub fn bounds(&self) -> Option<(Point, Point)> {
        let mut points = self.walls.iter().flat_map(|wall| [wall.start, wall.end]);
        let first = points.next()?;
        Some(points.fold((first, first), |(min, max), point| {
            (
                Point {
                    x: min.x.min(point.x),
                    y: min.y.min(point.y),
                },
                Point {
                    x: max.x.max(point.x),
                    y: max.y.max(point.y),
                },
            )
        }))
    }
}
