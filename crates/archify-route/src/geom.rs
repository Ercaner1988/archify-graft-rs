//! Plain geometry: points, rectangles and box sides.

pub type Pt = [f32; 2];

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    pub fn cx(&self) -> f32 {
        self.x + self.w * 0.5
    }

    pub fn cy(&self) -> f32 {
        self.y + self.h * 0.5
    }

    pub fn inflate(&self, d: f32) -> Rect {
        Rect::new(self.x - d, self.y - d, self.w + 2.0 * d, self.h + 2.0 * d)
    }

    /// True when the rectangles overlap by more than an edge (shared borders do not count).
    pub fn overlaps(&self, o: &Rect) -> bool {
        self.x < o.right() && o.x < self.right() && self.y < o.bottom() && o.y < self.bottom()
    }

    /// Strict interior test.
    pub fn contains_strict(&self, p: Pt) -> bool {
        p[0] > self.x && p[0] < self.right() && p[1] > self.y && p[1] < self.bottom()
    }

    pub fn union(&self, o: &Rect) -> Rect {
        let x = self.x.min(o.x);
        let y = self.y.min(o.y);
        Rect::new(
            x,
            y,
            self.right().max(o.right()) - x,
            self.bottom().max(o.bottom()) - y,
        )
    }

    /// Does the axis-aligned segment `a`-`b` pass through the interior?
    pub fn hit_by_segment(&self, a: Pt, b: Pt) -> bool {
        let (lo_x, hi_x) = (a[0].min(b[0]), a[0].max(b[0]));
        let (lo_y, hi_y) = (a[1].min(b[1]), a[1].max(b[1]));
        if (a[0] - b[0]).abs() < 1e-3 {
            a[0] > self.x && a[0] < self.right() && lo_y < self.bottom() && hi_y > self.y
        } else if (a[1] - b[1]).abs() < 1e-3 {
            a[1] > self.y && a[1] < self.bottom() && lo_x < self.right() && hi_x > self.x
        } else {
            // diagonal: conservative bounding-box test
            lo_x < self.right() && hi_x > self.x && lo_y < self.bottom() && hi_y > self.y
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

impl Side {
    pub fn is_horizontal(self) -> bool {
        matches!(self, Side::Left | Side::Right)
    }

    /// Unit vector pointing out of the box through this side.
    pub fn outward(self) -> Pt {
        match self {
            Side::Left => [-1.0, 0.0],
            Side::Right => [1.0, 0.0],
            Side::Top => [0.0, -1.0],
            Side::Bottom => [0.0, 1.0],
        }
    }

    pub fn opposite(self) -> Side {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
            Side::Top => Side::Bottom,
            Side::Bottom => Side::Top,
        }
    }

    /// Point of the side's middle.
    pub fn anchor(self, r: &Rect) -> Pt {
        match self {
            Side::Left => [r.x, r.cy()],
            Side::Right => [r.right(), r.cy()],
            Side::Top => [r.cx(), r.y],
            Side::Bottom => [r.cx(), r.bottom()],
        }
    }

    /// Length of the side (the room available for spreading ports).
    pub fn extent(self, r: &Rect) -> f32 {
        if self.is_horizontal() {
            r.h
        } else {
            r.w
        }
    }

    /// Port `offset` px along the side from its middle.
    pub fn port(self, r: &Rect, offset: f32) -> Pt {
        let a = self.anchor(r);
        if self.is_horizontal() {
            [a[0], a[1] + offset]
        } else {
            [a[0] + offset, a[1]]
        }
    }
}

pub fn dist(a: Pt, b: Pt) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}
