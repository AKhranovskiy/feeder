#[cfg(any(feature = "export-audio", feature = "export-image"))]
#[inline(always)]
pub(crate) fn ensure_dir_exists(name: &str) {
    let path = std::path::Path::new(name);
    if !path.try_exists().expect("Current directory is accesible") {
        // TODO create subdirs.
        std::fs::create_dir(path).expect("Can create subdir");
    }
}
