use std::io::Write;

#[no_mangle]
pub extern "C" fn export1() {
    std::io::stdout().write_all("export1".as_bytes()).unwrap();
    std::io::stdout().flush().unwrap();
}
