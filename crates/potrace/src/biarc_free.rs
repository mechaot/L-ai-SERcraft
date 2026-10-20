//! `PotraceClipper.Hilbert`: Hilbert curve generator (f32 like C#).

/// Hilbert curve helper (`Hilbert`).
pub struct Hilbert;

impl Hilbert {
    /// `Execute(depth, delta)`: `depth` recursion depth, `delta` step. Returns the points.
    pub fn execute(depth: i32, delta: f32) -> Vec<(f32, f32)> {
        let mut rv = vec![(0.0f32, 0.0f32)];
        let dy = 0.0f32;
        let dx = delta;
        Self::do_hilbert(&mut rv, depth, dx, dy);
        rv
    }

    /// `DoHilbert`: recursive. `list` output, `depth` level, `dx`/`dy` direction step.
    fn do_hilbert(list: &mut Vec<(f32, f32)>, depth: i32, dx: f32, dy: f32) {
        if depth > 1 {
            Self::do_hilbert(list, depth - 1, dy, dx);
        }
        let l = *list.last().expect("non-empty");
        list.push((l.0 + dx, l.1 + dy));
        if depth > 1 {
            Self::do_hilbert(list, depth - 1, dx, dy);
        }
        let l = *list.last().expect("non-empty");
        list.push((l.0 + dy, l.1 + dx));
        if depth > 1 {
            Self::do_hilbert(list, depth - 1, dx, dy);
        }
        let l = *list.last().expect("non-empty");
        list.push((l.0 - dx, l.1 - dy));
        if depth > 1 {
            Self::do_hilbert(list, depth - 1, -dy, -dx);
        }
    }
}
