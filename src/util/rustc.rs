use rustc_span::edition::Edition;

const RUSTC_EDITION: Edition = Edition::Edition2024;

pub fn init_rustc_globals<T>(f: impl FnOnce() -> T) -> T {
    rustc_span::create_session_globals_then(RUSTC_EDITION, &[], None, f)
}
