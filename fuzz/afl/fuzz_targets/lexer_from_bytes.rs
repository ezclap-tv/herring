#[macro_use]
extern crate afl;

use mu_lexer::_run_lexer;

fn main() {
  fuzz!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
      let _ = _run_lexer(&s);
    }
  });
}
