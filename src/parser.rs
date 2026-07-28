use crate::domain::{Opening, OpeningKind, Point, Project, Wall};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileError {
    pub line: usize,
    pub message: String,
}

pub fn compile(source: &str) -> Result<Project, CompileError> {
    let mut project = Project {
        name: "Untitled plan".into(),
        ..Project::default()
    };
    for (index, raw_line) in source.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with("//") || line.starts_with('#') {
            continue;
        }
        let line = line.trim_end_matches(';').trim();
        if line.starts_with("wall(") {
            let values = numbers(line);
            if values.len() < 4 {
                return Err(error(index, "wall expects [x1, y1], [x2, y2]"));
            }
            let thickness = named_number(line, "thickness").unwrap_or(300.0);
            let id = named_string(line, "id")
                .unwrap_or_else(|| format!("wall-{:03}", project.walls.len() + 1));
            if project.walls.iter().any(|wall| wall.id == id) {
                return Err(error(index, "wall id is already in use"));
            }
            project.walls.push(Wall {
                id,
                start: Point {
                    x: values[0],
                    y: values[1],
                },
                end: Point {
                    x: values[2],
                    y: values[3],
                },
                thickness,
            });
        } else if line.starts_with("door(") || line.starts_with("window(") {
            let wall_id = first_string(line)
                .ok_or_else(|| error(index, "opening needs a wall id as first argument"))?;
            if !project.walls.iter().any(|wall| wall.id == wall_id) {
                return Err(error(index, "opening references an unknown wall"));
            }
            let offset =
                named_number(line, "offset").ok_or_else(|| error(index, "opening needs offset"))?;
            let width =
                named_number(line, "width").ok_or_else(|| error(index, "opening needs width"))?;
            let wall = project
                .walls
                .iter()
                .find(|wall| wall.id == wall_id)
                .expect("wall was checked above");
            let wall_length = (wall.end.x - wall.start.x).hypot(wall.end.y - wall.start.y);
            if offset < 0.0 || width <= 0.0 || offset + width > wall_length {
                return Err(error(index, "opening must fit inside its wall"));
            }
            let is_window = line.starts_with("window(");
            let kind = if is_window {
                OpeningKind::Window {
                    sill: named_number(line, "sill").unwrap_or(900.0),
                }
            } else {
                OpeningKind::Door
            };
            project.openings.push(Opening {
                id: format!(
                    "{}-{:03}",
                    if is_window { "window" } else { "door" },
                    project.openings.len() + 1
                ),
                wall_id,
                offset,
                width,
                kind,
            });
        } else if line.starts_with("project(") {
            project.name = named_string(line, "name").unwrap_or_else(|| "Untitled plan".into());
        } else if !line.starts_with("view.") {
            return Err(error(
                index,
                "expected wall(...), door(...), window(...), or project(...)",
            ));
        }
    }
    Ok(project)
}

fn error(line: usize, message: impl Into<String>) -> CompileError {
    CompileError {
        line: line + 1,
        message: message.into(),
    }
}
fn numbers(input: &str) -> Vec<f64> {
    input
        .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse().ok())
        .collect()
}
fn named_number(input: &str, name: &str) -> Option<f64> {
    let (_, rest) = input.split_once(&format!("{name} ="))?;
    numbers(rest).first().copied()
}
fn first_string(input: &str) -> Option<String> {
    let (_, rest) = input.split_once('"')?;
    Some(rest.split_once('"')?.0.to_owned())
}
fn named_string(input: &str, name: &str) -> Option<String> {
    let (_, rest) = input.split_once(&format!("{name} = \""))?;
    Some(rest.split_once('"')?.0.to_owned())
}

#[cfg(test)]
mod tests {
    use super::compile;

    #[test]
    fn compiles_a_plan_with_openings() {
        let project = compile("wall([0, 0], [6000, 0], thickness = 300);\ndoor(\"wall-001\", offset = 1200, width = 900);").unwrap();
        assert_eq!(project.walls.len(), 1);
        assert_eq!(project.openings.len(), 1);
    }

    #[test]
    fn rejects_opening_without_wall() {
        assert!(compile("door(\"missing\", offset = 0, width = 900);").is_err());
    }
}
