#[cfg(feature = "export-audio")]
pub(crate) fn export_audio(data: &[u8], kind: &str) {
    use crate::util::ensure_dir_exists;
    use std::io::Write;

    ensure_dir_exists("audio");

    let hash = {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        Hash::hash_slice(data, &mut hasher);
        hasher.finish()
    };

    ensure_dir_exists(format!("audio/{kind}").as_str());

    std::fs::File::create(format!("./audio/{kind}/{hash:x}.aac"))
        .expect("Can create file")
        .write_all(data)
        .expect("File saved");
}

#[cfg(not(feature = "export-audio"))]
pub(crate) fn export_audio(_: &[u8], _: &str) {}
