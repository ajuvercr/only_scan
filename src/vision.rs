use rocket::serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Resp {
    pub responses: Vec<RespPart>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RespPart {
    text_annotations: Vec<TextAnnotation>,
}

const TRESH_HOLD: isize = 40;
impl RespPart {
    pub fn lines(&self) -> Vec<Vec<String>> {
        let mut out = Vec::new();

        let mut annotations: Vec<_> = self
            .text_annotations
            .iter()
            .map(TextAnnotation::with_center)
            .collect();
        annotations.sort_by_key(TextAnnotation::y);

        let mut sub = Vec::new();
        let mut current = None;
        for TextAnnotation {
            description,
            center: Point { x, y },
            ..
        } in annotations.into_iter()
        {
            let new_v = y;
            if let Some(cur) = current {
                if new_v - TRESH_HOLD < cur {
                    sub.push((x, description));
                } else {
                    out.push(sub);

                    sub = Vec::new();
                    sub.push((x, description));
                }
            } else {
                sub.push((x, description));
            }
            current = Some(new_v);
        }

        if !sub.is_empty() {
            out.push(sub);
        }

        out.into_iter()
            .map(|mut vec| {
                vec.sort_by_key(|(x, _)| *x);
                vec.into_iter().map(|(_, str)| str).collect()
            })
            .collect()
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TextAnnotation {
    bounding_poly: BoundingPoly,
    #[serde(skip)]
    center: Point,
    pub description: String,
}

impl TextAnnotation {
    fn with_center(&self) -> Self {
        Self {
            bounding_poly: self.bounding_poly.clone(),
            center: self.bounding_poly.center(),
            description: self.description.clone(),
        }
    }

    fn y(&self) -> isize {
        self.center.y
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BoundingPoly {
    vertices: [Point; 4],
}

impl BoundingPoly {
    pub fn center(&self) -> Point {
        Point::center(&self.vertices)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct Point {
    x: isize,
    y: isize,
}

impl Point {
    pub fn center(points: &[Point]) -> Point {
        if points.is_empty() {
            return Point::default();
        }
        let Point { x, y } = points.iter().fold(
            Point::default(),
            |Point { x: x1, y: y1 }, Point { x: x2, y: y2 }| Point {
                x: x1 + x2,
                y: y1 + y2,
            },
        );

        Point {
            x: x / points.len() as isize,
            y: y / points.len() as isize,
        }
    }
}

impl Default for Point {
    fn default() -> Self {
        Self { x: 0, y: 0 }
    }
}
