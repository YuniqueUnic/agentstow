use std::net::SocketAddr;

use assert_fs::prelude::*;
use rstest::{fixture, rstest};

#[fixture]
pub fn fixture() -> u32 {
    42
}

#[rstest]
fn should_success(fixture: u32) {
    assert_eq!(fixture, 42);
}

#[rstest]
#[should_panic(expected = "assertion `left != right` failed")]
fn should_fail(fixture: u32) {
    assert_ne!(fixture, 42);
}

#[rstest]
#[case("1.2.3.4:8080", 8080)]
#[case("127.0.0.1:9000", 9000)]
fn check_port(#[case] addr: SocketAddr, #[case] expected: u16) {
    // only for pass the clippy warning: warning: unused import: `assert_fs::prelude::*`
    let temp = assert_fs::TempDir::new().unwrap();
    let _input_file = temp.child("foo.txt");
    assert_eq!(expected, addr.port());
}
