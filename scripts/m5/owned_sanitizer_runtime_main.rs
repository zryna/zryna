// Fixed append-only entry for a source-exact driver mirror compiled as bin with --cfg test.
fn main() {
    let mut arguments = std::env::args_os().skip(1);
    let parent = std::path::PathBuf::from(arguments.next().expect("one owned evidence parent"));
    assert!(arguments.next().is_none(), "no alternate flags or selectors");
    match m5_owned_sanitizer_runtime(&parent) {
        Ok(report) => {
            let bytes = serde_json::to_vec(&report).expect("complete ordinary runtime report");
            assert!(bytes.len() <= 1024 * 1024);
            println!("{}", std::str::from_utf8(&bytes).expect("JSON UTF-8"));
        }
        Err(error) => {
            eprintln!("owned ordinary sanitizer runtime unavailable: {error:?}");
            std::process::exit(1);
        }
    }
}
