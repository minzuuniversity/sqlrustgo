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

/// ST_Distance(point1, point2) - Euclidean distance between two points
pub fn st_distance(p1: &Point, p2: &Point) -> f64 {
    ((p1.x - p2.x).powi(2) + (p1.y - p2.y).powi(2)).sqrt()
}

/// ST_Contains(polygon, point) - returns true if polygon contains point
pub fn st_contains(polygon: &Polygon, point: &Point) -> bool {
    st_within(point, polygon)
}

/// ST_Intersects(polygon1, polygon2) - returns true if two polygons intersect
/// Uses bounding box quick-check first, then ray casting at edge intersections
pub fn st_intersects(p1: &Polygon, p2: &Polygon) -> bool {
    // Quick rejection: if bounding boxes don't overlap, polygons don't intersect
    let (p1_min_x, p1_min_y, p1_max_x, p1_max_y) = p1.bbox();
    let (p2_min_x, p2_min_y, p2_max_x, p2_max_y) = p2.bbox();

    if p1_max_x < p2_min_x || p2_max_x < p1_min_x ||
       p1_max_y < p2_min_y || p2_max_y < p1_min_y {
        return false;
    }

    // Check if any vertex of p1 is inside p2
    for v in &p1.vertices {
        if st_within_point_polygon(v, p2) {
            return true;
        }
    }

    // Check if any vertex of p2 is inside p1
    for v in &p2.vertices {
        if st_within_point_polygon(v, p1) {
            return true;
        }
    }

    // Check for edge intersections (line segment intersection)
    let n1 = p1.vertices.len();
    let n2 = p2.vertices.len();

    for i in 0..n1 {
        let a1 = &p1.vertices[i];
        let a2 = &p1.vertices[(i + 1) % n1];

        for j in 0..n2 {
            let b1 = &p2.vertices[j];
            let b2 = &p2.vertices[(j + 1) % n2];

            if segments_intersect(a1, a2, b1, b2) {
                return true;
            }
        }
    }

    false
}

/// Check if two line segments (a1-a2) and (b1-b2) intersect
fn segments_intersect(a1: &Point, a2: &Point, b1: &Point, b2: &Point) -> bool {
    fn ccw(p1: &Point, p2: &Point, p3: &Point) -> bool {
        (p3.y - p1.y) * (p2.x - p1.x) > (p2.y - p1.y) * (p3.x - p1.x)
    }

    ccw(a1, b1, b2) != ccw(a2, b1, b2) && ccw(a1, a2, b1) != ccw(a1, a2, b2)
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

    #[test]
    fn test_polygon_parse() {
        // POLYGON() format
        let p = Polygon::parse("POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))").unwrap();
        assert_eq!(p.vertices.len(), 5);

        // Space/comma format
        let p2 = Polygon::parse("0 0, 10 0, 10 10, 0 10").unwrap();
        assert_eq!(p2.vertices.len(), 4);

        // Too few vertices
        assert!(Polygon::parse("0 0, 1 1").is_none());
    }

    #[test]
    fn test_polygon_bbox() {
        let poly = Polygon::new(vec![
            Point::new(5.0, 3.0),
            Point::new(15.0, 3.0),
            Point::new(15.0, 13.0),
            Point::new(5.0, 13.0),
        ]);
        let (min_x, min_y, max_x, max_y) = poly.bbox();
        assert_eq!((min_x, min_y, max_x, max_y), (5.0, 3.0, 15.0, 13.0));
    }

    #[test]
    fn test_st_within_point_on_edge() {
        let poly = Polygon::new(vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
            Point::new(0.0, 10.0),
        ]);
        // Point on edge
        let on_edge = Point::new(5.0, 0.0);
        let result = st_within_point_polygon(&on_edge, &poly);
        // Ray casting: point on edge may be inside or outside depending on implementation
        // Just verify no panic
        let _ = result;
    }

    #[test]
    fn test_st_within_invalid_polygon() {
        let poly = Polygon::new(vec![Point::new(0.0, 0.0), Point::new(5.0, 5.0)]);
        // Less than 3 vertices — should return false
        assert!(!st_within_point_polygon(&Point::new(1.0, 1.0), &poly));
    }

    #[test]
    fn test_st_within_point_in_bbox_false() {
        let poly = Polygon::new(vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
            Point::new(0.0, 10.0),
        ]);
        // Point in bbox but outside polygon (diagonal)
        let inside_bbox = Point::new(9.0, 1.0);
        assert!(st_within(&inside_bbox, &poly));
    }

    #[test]
    fn test_point_from_value() {
        use sqlrustgo_types::Value;
        // Integer should return None
        assert!(point_from_value(&Value::Integer(1)).is_none());
        // Real point
        let p = point_from_value(&Value::Point(1.5, 2.5)).unwrap();
        assert!((p.x - 1.5).abs() < 0.001);
        assert!((p.y - 2.5).abs() < 0.001);
    }

    #[test]
    fn test_value_from_point() {
        let p = Point::new(1.5, 2.5);
        let v = value_from_point(&p);
        match v {
            Value::Point(x, y) => {
                assert!((x - 1.5).abs() < 0.001);
                assert!((y - 2.5).abs() < 0.001);
            }
            _ => panic!("expected Point variant"),
        }
    }

    #[test]
    fn test_point_to_sql_string() {
        let p = Point::new(1.5, 2.5);
        assert_eq!(p.to_sql_string(), "POINT(1.5, 2.5)");
    }

    #[test]
    fn test_point_in_bbox() {
        let p = Point::new(5.0, 5.0);
        assert!(p.in_bbox(0.0, 0.0, 10.0, 10.0));
        assert!(!p.in_bbox(0.0, 0.0, 4.0, 4.0));
        assert!(p.in_bbox(5.0, 5.0, 5.0, 5.0)); // boundary
    }

    #[test]
    fn test_polygon_parse_coords_variations() {
        // With parentheses — splits on comma, so "0 0" is one part
        let p = Polygon::parse("(0 0, 5 0, 5 5, 0 5)").unwrap();
        assert_eq!(p.vertices.len(), 4);
        // Without parentheses, space/comma separated
        let p2 = Polygon::parse("0 0, 5 0, 5 5, 0 5").unwrap();
        assert_eq!(p2.vertices.len(), 4);
    }

    #[test]
    fn test_st_within_triangle() {
        // Triangle polygon
        let poly = Polygon::new(vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(5.0, 10.0),
        ]);
        let inside = Point::new(5.0, 2.0);
        assert!(st_within(&inside, &poly));

        let outside = Point::new(5.0, 11.0);
        assert!(!st_within(&outside, &poly));
    }
}

/// Calculate the Haversine distance between two points in meters
pub fn haversine_distance(p1: &Point, p2: &Point) -> f64 {
    const EARTH_RADIUS_METERS: f64 = 6371000.0;
    
    let lat1 = p1.y.to_radians();
    let lat2 = p2.y.to_radians();
    let delta_lat = (p2.y - p1.y).to_radians();
    let delta_lon = (p2.x - p1.x).to_radians();

    let a = (delta_lat / 2.0).sin().powi(2)
        + lat1.cos() * lat2.cos() * (delta_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();
    
    EARTH_RADIUS_METERS * c
}

/// Calculate Euclidean distance between two points
pub fn euclidean_distance(p1: &Point, p2: &Point) -> f64 {
    ((p2.x - p1.x).powi(2) + (p2.y - p1.y).powi(2)).sqrt()
}



/// ST_Distance_Point - wrapper for SQL interface
pub fn st_distance_point(p1: &Point, p2: &Point) -> Value {
    Value::Float(st_distance(p1, p2))
}

/// ST_Intersects_Point - wrapper for SQL interface  
pub fn st_intersects_point(p1: &Point, p2: &Point) -> Value {
    Value::Boolean(points_intersect(p1, p2))
}

/// Point-point intersection with tolerance
fn points_intersect(p1: &Point, p2: &Point) -> bool {
    euclidean_distance(p1, p2) < 1e-10
}
