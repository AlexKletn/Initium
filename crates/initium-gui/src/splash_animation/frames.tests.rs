use std::io::{Cursor, ErrorKind};

use super::read_frame;

#[test]
fn decodes_complete_frames_and_detects_end_of_stream() {
    let bytes = [
        2, 0, 0, 0, 1, 2, 3, 255, 1, 0, 0, 0, 4, 5, 6, 128, 3, 0, 0, 0, 0, 0, 0, 0,
    ];
    let mut source = Cursor::new(bytes);
    let mut pixels = [0; 3];
    assert!(read_frame(&mut source, &mut pixels).unwrap());
    assert_eq!(pixels, [0xff030201, 0xff030201, 0x80060504]);
    assert!(read_frame(&mut source, &mut pixels).unwrap());
    assert_eq!(pixels, [0; 3]);
    assert!(!read_frame(&mut source, &mut pixels).unwrap());
}

#[test]
fn rejects_a_run_that_overflows_the_frame() {
    let mut source = Cursor::new([4, 0, 0, 0, 1, 2, 3, 255]);
    let error = read_frame(&mut source, &mut [0; 3]).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::InvalidData);
}
