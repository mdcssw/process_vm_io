#[test]
fn min_system_page_size() {
    let size = super::min_system_page_size().unwrap();
    assert!(size.get() < u64::MAX);
    assert!(size.is_power_of_two());
}
