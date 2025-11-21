use gl_generator::{Api, Fallbacks, Profile, Registry, StructGenerator};
use std::env;
use std::fs::File;
use std::path::Path;

fn main() {
    let dest = env::var("OUT_DIR").unwrap();
    let mut file = File::create(Path::new(&dest).join("gl_bindings.rs")).unwrap();

    Registry::new(
        Api::Gles2,
        (3, 3),
        Profile::Core,
        Fallbacks::All,
        [
            "GL_OES_EGL_image",
            "GL_OES_EGL_image_external",
            "GL_EXT_memory_object_fd",
            "GL_EXT_semaphore_fd",
        ],
    )
    .write_bindings(StructGenerator, &mut file)
    .unwrap();
}
