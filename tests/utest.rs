include!("../src/pio.rs");
#[test]
fn test_pio() {
    let mut buf : [u8;1024] = [0;1024];

    let mut pio = Pio::new();

    pio.set(String::from("/bin/sh"), String::from(""));
    pio.run();

    pio.write("echo test1\n".as_bytes());
    let result = pio.read(buf.as_mut());
    assert_eq!(std::str::from_utf8(&buf[..result.unwrap()]).unwrap(), "test1\n");

    buf.fill(0);

    pio.write("echo test2\n".as_bytes());
    let result = pio.read(buf.as_mut());
    assert_eq!(std::str::from_utf8(&buf[..result.unwrap()]).unwrap(), "test2\n");
}