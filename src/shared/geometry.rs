//! Symmetric resize geometry: the grabbed edge follows the pointer and the
//! opposite edge mirrors it, so the window grows or shrinks around its centre.
//!
//! Units are whatever the platform uses for window rectangles (physical pixels
//! on Windows); y grows downwards.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub fn width(&self) -> i32 {
        self.right - self.left
    }

    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

/// Which edges the user grabbed (a corner grabs two).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Edges {
    pub left: bool,
    pub top: bool,
    pub right: bool,
    pub bottom: bool,
}

/// Smallest and largest size the window accepts, as (width, height).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub min: (i32, i32),
    pub max: (i32, i32),
}

/// The window rectangle once the pointer has moved by (`dx`, `dy`) since the
/// drag started on `edges` of `start`.
pub fn symmetric(start: Rect, edges: Edges, dx: i32, dy: i32, limits: Limits) -> Rect {
    // Dragging an edge outwards by d grows the window by d on both sides.
    let grow = |delta: i32, low: bool, high: bool| if high { delta } else if low { -delta } else { 0 };
    let width = start.width() + 2 * grow(dx, edges.left, edges.right);
    let height = start.height() + 2 * grow(dy, edges.top, edges.bottom);
    around_centre(
        start,
        edges,
        clamp(width, limits.min.0, limits.max.0),
        clamp(height, limits.min.1, limits.max.1),
    )
}

/// `start` resized to `width` × `height`, keeping its centre on the axes being
/// resized and its position on the others.
pub fn around_centre(start: Rect, edges: Edges, width: i32, height: i32) -> Rect {
    let place = |low: i32, high: i32, size: i32, resized: bool| {
        let low = if resized { low + (high - low - size).div_euclid(2) } else { low };
        (low, low + size)
    };
    let (left, right) = place(start.left, start.right, width, edges.left || edges.right);
    let (top, bottom) = place(start.top, start.bottom, height, edges.top || edges.bottom);
    Rect { left, top, right, bottom }
}

fn clamp(size: i32, min: i32, max: i32) -> i32 {
    let min = min.max(1);
    size.clamp(min, max.max(min))
}

#[cfg(test)]
mod tests {
    use super::*;

    const START: Rect = Rect { left: 100, top: 200, right: 500, bottom: 500 };
    const LOOSE: Limits = Limits { min: (50, 40), max: (10_000, 10_000) };
    const LEFT: Edges = Edges { left: true, top: false, right: false, bottom: false };
    const RIGHT: Edges = Edges { left: false, top: false, right: true, bottom: false };
    const TOP: Edges = Edges { left: false, top: true, right: false, bottom: false };
    const BOTTOM_RIGHT: Edges = Edges { left: false, top: false, right: true, bottom: true };

    fn rect(left: i32, top: i32, right: i32, bottom: i32) -> Rect {
        Rect { left, top, right, bottom }
    }

    #[test]
    fn dragging_the_right_edge_out_moves_the_left_edge_out_too() {
        assert_eq!(symmetric(START, RIGHT, 50, 0, LOOSE), rect(50, 200, 550, 500));
    }

    #[test]
    fn dragging_the_left_edge_in_pulls_the_right_edge_in_too() {
        assert_eq!(symmetric(START, LEFT, 30, 0, LOOSE), rect(130, 200, 470, 500));
    }

    #[test]
    fn vertical_motion_is_ignored_on_a_side_edge() {
        assert_eq!(symmetric(START, RIGHT, 20, 80, LOOSE), rect(80, 200, 520, 500));
    }

    #[test]
    fn the_top_edge_resizes_vertically_only() {
        assert_eq!(symmetric(START, TOP, 40, -25, LOOSE), rect(100, 175, 500, 525));
    }

    #[test]
    fn a_corner_resizes_both_axes() {
        assert_eq!(symmetric(START, BOTTOM_RIGHT, 10, 20, LOOSE), rect(90, 180, 510, 520));
    }

    #[test]
    fn the_centre_never_moves() {
        for (dx, dy) in [(-7, 3), (13, -21), (150, 99), (-180, -140)] {
            let r = symmetric(START, BOTTOM_RIGHT, dx, dy, LOOSE);
            assert_eq!((r.left + r.right, r.top + r.bottom), (600, 700), "dx={dx} dy={dy}");
        }
    }

    #[test]
    fn shrinking_stops_at_the_minimum_size_and_stays_centred() {
        let r = symmetric(START, RIGHT, -1000, 0, LOOSE);
        assert_eq!(r, rect(275, 200, 325, 500));
    }

    #[test]
    fn growing_stops_at_the_maximum_size() {
        let limits = Limits { min: (50, 40), max: (600, 400) };
        assert_eq!(symmetric(START, BOTTOM_RIGHT, 500, 500, limits), rect(0, 150, 600, 550));
    }

    #[test]
    fn odd_sizes_round_consistently() {
        assert_eq!(around_centre(START, RIGHT, 401, 300), rect(99, 200, 500, 500));
        assert_eq!(around_centre(START, RIGHT, 399, 300), rect(100, 200, 499, 500));
    }

    #[test]
    fn an_axis_that_is_not_being_resized_keeps_its_position() {
        // e.g. the app changed its own height while the user drags a side edge
        assert_eq!(around_centre(START, LEFT, 400, 320), rect(100, 200, 500, 520));
    }
}
