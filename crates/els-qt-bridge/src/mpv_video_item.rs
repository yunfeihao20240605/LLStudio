pub fn register_qml_type() {
    unsafe {
        els_register_mpv_video_item();
    }
}

unsafe extern "C" {
    fn els_register_mpv_video_item();
}
