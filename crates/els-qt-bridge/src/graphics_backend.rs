pub fn force_opengl_backend() {
    unsafe {
        els_force_opengl_backend();
    }
}

unsafe extern "C" {
    fn els_force_opengl_backend();
}
