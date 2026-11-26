use std::fs::File;

use super::*;

#[test]
fn keeps_a_duplicate_of_the_new_device_descriptor() {
    let path = Path::new("/dev/null");
    let previous = device_descriptors(path).expect("list device descriptors");
    let original = File::open(path).expect("open device");
    let master = DrmMaster::capture(path, &previous, 42).expect("duplicate the new descriptor");

    assert_ne!(master.fd.as_raw_fd(), original.as_raw_fd());
    drop(original);
    assert!(
        device_descriptors(path)
            .expect("list new descriptors")
            .contains(&master.fd.as_raw_fd())
    );
}
