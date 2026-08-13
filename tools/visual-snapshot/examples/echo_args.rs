//! Minimal fixture binary for `Session::spawn_with_args` tests: exits
//! 0 if invoked with exactly the args `["hello", "world"]`, exits 1
//! otherwise.
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args == vec!["hello".to_string(), "world".to_string()] {
        std::process::exit(0);
    }
    std::process::exit(1);
}
