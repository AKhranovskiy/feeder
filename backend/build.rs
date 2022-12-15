fn main() {
    println!("cargo:rustc-env=LD_LIBRARY_PATH={}/lib/", env!("LIBTORCH"));
}
