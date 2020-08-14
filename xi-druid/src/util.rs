use xi_rope::compare::RopeScanner;
use xi_rope::Rope;

// TODO: this functionality should be moved to xi-rope.
pub fn rope_eq(a: &Rope, b: &Rope) -> bool {
    let len = a.len();
    if len != b.len() {
        return false;
    }
    RopeScanner::new(a, b).find_ne_char(0, 0, None) == len
}
