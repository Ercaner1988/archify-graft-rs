//! Straight (non-premultiplied) RGBA colour with CSS round-trips.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Rgba {
        Rgba { r, g, b, a: 255 }
    }

    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Rgba {
        Rgba { r, g, b, a }
    }

    /// Parses `#rgb`, `#rrggbb` or `rgba(r,g,b,a)` / `rgb(r,g,b)`. Unknown input is magenta so
    /// a typo in a palette table is visible instead of silent.
    pub fn parse(s: &str) -> Rgba {
        let s = s.trim();
        if let Some(h) = s.strip_prefix('#') {
            return parse_hex(h).unwrap_or(Rgba::rgb(255, 0, 255));
        }
        if let Some(body) = s
            .strip_prefix("rgba(")
            .or_else(|| s.strip_prefix("rgb("))
            .and_then(|r| r.strip_suffix(')'))
        {
            let mut it = body.split(',').map(str::trim);
            let mut chan = || it.next().and_then(|v| v.parse::<f32>().ok());
            if let (Some(r), Some(g), Some(b)) = (chan(), chan(), chan()) {
                let a = it.next().and_then(|v| v.parse::<f32>().ok()).unwrap_or(1.0);
                return Rgba::new(r as u8, g as u8, b as u8, (a * 255.0).round() as u8);
            }
        }
        Rgba::rgb(255, 0, 255)
    }

    /// Same colour with alpha scaled by `k` (0..1).
    pub fn scale_alpha(self, k: f32) -> Rgba {
        Rgba {
            a: (self.a as f32 * k.clamp(0.0, 1.0)).round() as u8,
            ..self
        }
    }

    pub fn with_alpha(self, a: f32) -> Rgba {
        Rgba {
            a: (a.clamp(0.0, 1.0) * 255.0).round() as u8,
            ..self
        }
    }

    /// Linear mix towards `other` (t = 0 keeps self), alpha included.
    pub fn mix(self, other: Rgba, t: f32) -> Rgba {
        let l = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
        Rgba::new(
            l(self.r, other.r),
            l(self.g, other.g),
            l(self.b, other.b),
            l(self.a, other.a),
        )
    }

    /// `#rrggbb`, alpha dropped.
    pub fn hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// `#rrggbb` when opaque, otherwise `rgba(r,g,b,a)`; valid in SVG attributes and CSS.
    pub fn css(self) -> String {
        if self.a == 255 {
            self.hex()
        } else {
            format!(
                "rgba({},{},{},{})",
                self.r,
                self.g,
                self.b,
                trim_float(self.a as f32 / 255.0)
            )
        }
    }

    pub fn alpha_f(self) -> f32 {
        self.a as f32 / 255.0
    }
}

fn parse_hex(h: &str) -> Option<Rgba> {
    let v = |s: &str| u8::from_str_radix(s, 16).ok();
    match h.len() {
        6 => Some(Rgba::rgb(v(&h[0..2])?, v(&h[2..4])?, v(&h[4..6])?)),
        3 => {
            let d = |i: usize| v(&h[i..i + 1]).map(|n| n * 17);
            Some(Rgba::rgb(d(0)?, d(1)?, d(2)?))
        }
        _ => None,
    }
}

fn trim_float(x: f32) -> String {
    let s = format!("{x:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() {
        "0".to_string()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_and_rgba() {
        assert_eq!(Rgba::parse("#22d3ee"), Rgba::rgb(0x22, 0xd3, 0xee));
        assert_eq!(Rgba::parse("#fff"), Rgba::rgb(255, 255, 255));
        let c = Rgba::parse("rgba(8,51,68,.4)");
        assert_eq!((c.r, c.g, c.b, c.a), (8, 51, 68, 102));
    }

    #[test]
    fn css_round_trip() {
        assert_eq!(Rgba::rgb(1, 2, 3).css(), "#010203");
        assert_eq!(Rgba::parse("rgba(8,51,68,.4)").css(), "rgba(8,51,68,0.4)");
    }
}
