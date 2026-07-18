//! GIS - Geographic Information System module
//!
//! Provides POINT data type and spatial functions like ST_WITHIN.

use sqlrustgo_types::Value;

/// A geographic point (x, y) = (longitude, latitude)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64, // longitude
    pub y: f64, // latitude
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Parse from MySQL-compatible POINT(x, y) string
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if s.to_uppercase().starts_with("POINT(") && s.ends_with(')') {
            let inner = &s[6..s.len() - 1];
            return Self::parse_coords(inner);
        }
        Self::parse_coords(s)
    }

    fn parse_coords(s: &str) -> Option<Self> {
        let s = s.trim().trim_start_matches('(').trim_end_matches(')');
        let parts: Vec<&str> = if s.contains(',') {
            s.split(',').collect()
        } else {
            s.split_whitespace().collect()
        };

        if parts.len() >= 2 {
            let x = parts[0].trim().parse::<f64>().ok()?;
            let y = parts[1].trim().parse::<f64>().ok()?;
            Some(Self::new(x, y))
        } else {
            None
        }
    }

    pub fn to_sql_string(&self) -> String {
        format!("POINT({}, {})", self.x, self.y)
    }

    pub fn in_bbox(&self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> bool {
        self.x >= min_x && self.x <= max_x && self.y >= min_y && self.y <= max_y
    }
}

/// A polygon defined by its vertices
#[derive(Debug, Clone)]
pub struct Polygon {
    pub vertices: Vec<Point>,
}

impl Polygon {
    pub fn new(vertices: Vec<Point>) -> Self {
        Self { vertices }
    }

    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        let inner = if s.to_uppercase().starts_with("POLYGON(") && s.ends_with(')') {
            &s[8..s.len() - 1]
        } else {
            s
        };

        let mut vertices = Vec::new();
        for pair in inner.split(',') {
            if let Some(point) = Point::parse(pair) {
                vertices.push(point);
            }
        }

        if vertices.len() >= 3 {
            Some(Self::new(vertices))
        } else {
            None
        }
    }

    pub fn bbox(&self) -> (f64, f64, f64, f64) {
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for v in &self.vertices {
            min_x = min_x.min(v.x);
            min_y = min_y.min(v.y);
            max_x = max_x.max(v.x);
            max_y = max_y.max(v.y);
        }

        (min_x, min_y, max_x, max_y)
    }
}

/// Ray casting point-in-polygon algorithm
pub fn st_within_point_polygon(point: &Point, polygon: &Polygon) -> bool {
    let n = polygon.vertices.len();
    if n < 3 {
        return false;
    }

    let (min_x, min_y, max_x, max_y) = polygon.bbox();
    if !point.in_bbox(min_x, min_y, max_x, max_y) {
        return false;
    }

    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let vi = &polygon.vertices[i];
        let vj = &polygon.vertices[j];

        if ((vi.y > point.y) != (vj.y > point.y))
            && (point.x < (vj.x - vi.x) * (point.y - vi.y) / (vj.y - vi.y) + vi.x)
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// ST_WITHIN(point, polygon) - returns true if point is within polygon
pub fn st_within(point: &Point, polygon: &Polygon) -> bool {
    st_within_point_polygon(point, polygon)
}

/// Create Point from Value::Point
pub fn point_from_value(value: &Value) -> Option<Point> {
    match value {
        Value::Point(x, y) => Some(Point::new(*x, *y)),
        _ => None,
    }
}

/// Create Value::Point from Point
pub fn value_from_point(point: &Point) -> Value {
    Value::Point(point.x, point.y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_parse_mysql() {
        let p = Point::parse("POINT(40.7128, -74.0060)").unwrap();
        assert!((p.x - 40.7128).abs() < 0.0001);
        assert!((p.y - (-74.0060)).abs() < 0.0001);
    }

    #[test]
    fn test_point_parse_coords() {
        let p = Point::parse("40.7128, -74.0060").unwrap();
        assert!((p.x - 40.7128).abs() < 0.0001);
        assert!((p.y - (-74.0060)).abs() < 0.0001);
    }

    #[test]
    fn test_st_within_inside() {
        let poly = Polygon::new(vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
            Point::new(0.0, 10.0),
        ]);

        let inside = Point::new(5.0, 5.0);
        assert!(st_within(&inside, &poly));

        let outside = Point::new(15.0, 15.0);
        assert!(!st_within(&outside, &poly));
    }
}
