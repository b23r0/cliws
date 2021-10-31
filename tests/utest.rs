include!("../src/utils.rs");
#[test]
fn test_get_termsize() {
    let a = get_termsize().unwrap();
    assert!(a.ws_row != 0);
    assert!(a.ws_col != 0);
}
#[test]
fn test_set_termsize() {
    let size = Box::new(libc::winsize{
        ws_row : 50, 
        ws_col :  50,
        ws_xpixel : 0,
        ws_ypixel: 0, 
        
    });
    set_termsize(size);
}