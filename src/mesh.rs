use std::fmt::Write;

use crate::domain::{OpeningKind, Point, Project, Wall};

/// Generates a compact OBJ preview from the canonical plan document.
#[must_use]
pub fn project_to_obj(project: &Project) -> Vec<u8> {
    let mut obj = String::from("o baumaus-plan\n");
    let mut next_vertex = 1;
    let mut next_normal = 1;
    for wall in &project.walls {
        for (start, end) in wall_segments(project, wall) {
            append_prism(
                &mut obj,
                &mut next_vertex,
                &mut next_normal,
                start,
                end,
                wall.thickness,
                (0.0, 3000.0),
            );
        }
        for opening in project
            .openings
            .iter()
            .filter(|opening| opening.wall_id == wall.id)
        {
            let OpeningKind::Window { sill } = &opening.kind else {
                continue;
            };
            let length = (wall.end.x - wall.start.x).hypot(wall.end.y - wall.start.y);
            let start = point_at(wall, opening.offset / length);
            let end = point_at(wall, (opening.offset + opening.width) / length);
            append_prism(
                &mut obj,
                &mut next_vertex,
                &mut next_normal,
                start,
                end,
                wall.thickness,
                (0.0, *sill),
            );
            append_prism(
                &mut obj,
                &mut next_vertex,
                &mut next_normal,
                start,
                end,
                wall.thickness,
                (*sill + 1200.0, 3000.0),
            );
        }
    }
    obj.into_bytes()
}

fn append_prism(
    obj: &mut String,
    next_vertex: &mut usize,
    next_normal: &mut usize,
    start: Point,
    end: Point,
    thickness: f64,
    height: (f64, f64),
) {
    let (bottom, top) = height;
    if bottom >= top {
        return;
    }
    let Some([a, b, c, d]) = footprint(start, end, thickness) else {
        return;
    };
    let base = *next_vertex;
    let normal_base = *next_normal;
    for point in [a, b, c, d] {
        let _ = writeln!(
            obj,
            "v {} {} {}",
            point.x / 1000.0,
            bottom / 1000.0,
            point.y / 1000.0
        );
    }
    for point in [a, b, c, d] {
        let _ = writeln!(
            obj,
            "v {} {} {}",
            point.x / 1000.0,
            top / 1000.0,
            point.y / 1000.0
        );
    }
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length = dx.hypot(dy);
    let left = (-dy / length, dx / length);
    let _ = writeln!(
        obj,
        "vn 0 -1 0\nvn 0 1 0\nvn {} 0 {}\nvn {} 0 {}\nvn {} 0 {}\nvn {} 0 {}",
        left.0,
        left.1,
        dx / length,
        dy / length,
        -left.0,
        -left.1,
        -dx / length,
        -dy / length
    );
    let _ = write!(obj, "f {base}//{normal_base} {}//{normal_base} {}//{normal_base} {}//{normal_base}\nf {}//{} {}//{} {}//{} {}//{}\nf {base}//{} {}//{} {}//{} {}//{}\nf {}//{} {}//{} {}//{} {}//{}\nf {}//{} {}//{} {}//{} {}//{}\nf {}//{} {base}//{} {}//{} {}//{}\n", base + 1, base + 2, base + 3, base + 4, normal_base + 1, base + 5, normal_base + 1, base + 6, normal_base + 1, base + 7, normal_base + 1, normal_base + 2, base + 1, normal_base + 2, base + 5, normal_base + 2, base + 4, normal_base + 2, base + 1, normal_base + 3, base + 2, normal_base + 3, base + 6, normal_base + 3, base + 5, normal_base + 3, base + 2, normal_base + 4, base + 3, normal_base + 4, base + 7, normal_base + 4, base + 6, normal_base + 4, base + 3, normal_base + 5, normal_base + 5, base + 4, normal_base + 5, base + 7, normal_base + 5);
    *next_vertex += 8;
    *next_normal += 6;
}

fn wall_segments(project: &Project, wall: &Wall) -> Vec<(Point, Point)> {
    let length = (wall.end.x - wall.start.x).hypot(wall.end.y - wall.start.y);
    if length <= f64::EPSILON {
        return Vec::new();
    }
    let mut openings: Vec<(f64, f64)> = project
        .openings
        .iter()
        .filter(|opening| opening.wall_id == wall.id)
        .map(|opening| {
            (
                opening.offset.max(0.0),
                (opening.offset + opening.width).min(length),
            )
        })
        .filter(|(start, end)| end > start)
        .collect();
    openings.sort_by(|left, right| left.0.total_cmp(&right.0));

    let mut segments = Vec::new();
    let mut cursor = 0.0;
    for (start, end) in openings {
        if start > cursor {
            segments.push((
                point_at(wall, cursor / length),
                point_at(wall, start / length),
            ));
        }
        cursor = cursor.max(end);
    }
    if cursor < length {
        segments.push((point_at(wall, cursor / length), point_at(wall, 1.0)));
    }
    segments
}

fn point_at(wall: &Wall, ratio: f64) -> Point {
    Point {
        x: wall.start.x + (wall.end.x - wall.start.x) * ratio,
        y: wall.start.y + (wall.end.y - wall.start.y) * ratio,
    }
}

fn footprint(start: Point, end: Point, thickness: f64) -> Option<[Point; 4]> {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length = dx.hypot(dy);
    if length <= f64::EPSILON || thickness <= 0.0 {
        return None;
    }
    let scale = thickness / (2.0 * length);
    let normal = Point {
        x: -dy * scale,
        y: dx * scale,
    };
    Some([
        Point {
            x: start.x + normal.x,
            y: start.y + normal.y,
        },
        Point {
            x: end.x + normal.x,
            y: end.y + normal.y,
        },
        Point {
            x: end.x - normal.x,
            y: end.y - normal.y,
        },
        Point {
            x: start.x - normal.x,
            y: start.y - normal.y,
        },
    ])
}

#[cfg(test)]
mod tests {
    use crate::{
        domain::{Point, Project, Wall},
        mesh::project_to_obj,
    };

    #[test]
    fn emits_faces_for_a_wall() {
        let project = Project {
            walls: vec![Wall {
                id: "wall-001".into(),
                start: Point { x: 0.0, y: 0.0 },
                end: Point { x: 1000.0, y: 0.0 },
                thickness: 200.0,
            }],
            ..Project::default()
        };
        let obj = String::from_utf8(project_to_obj(&project)).unwrap();
        assert_eq!(obj.matches("\nv ").count(), 8);
        assert_eq!(obj.matches("\nvn ").count(), 6);
        assert_eq!(obj.matches("\nf ").count(), 6);
    }

    #[test]
    fn cuts_an_opening_out_of_a_wall() {
        let mut project = Project {
            walls: vec![Wall {
                id: "wall-001".into(),
                start: Point { x: 0.0, y: 0.0 },
                end: Point { x: 4000.0, y: 0.0 },
                thickness: 200.0,
            }],
            ..Project::default()
        };
        project.openings.push(crate::domain::Opening {
            id: "door-001".into(),
            wall_id: "wall-001".into(),
            offset: 1000.0,
            width: 900.0,
            kind: crate::domain::OpeningKind::Door,
        });
        let obj = String::from_utf8(project_to_obj(&project)).unwrap();
        assert_eq!(obj.matches("\nf ").count(), 12);
    }
}
