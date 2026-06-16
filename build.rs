use std::env;
use std::fs;
use std::os::unix::fs as unix_fs;
use std::path::Path;

fn main() {
    let target = env::var("TARGET").unwrap_or_default();
    if target == "aarch64-linux-android" {
        let out_dir = env::var("OUT_DIR").unwrap();
        let stub = Path::new(&out_dir).join("libunwind.a");
        let gcc = "/android-ndk/lib/gcc/aarch64-linux-android/4.9.x/libgcc.a";
        if Path::new(gcc).exists() {
            let _ = fs::remove_file(&stub);
            let _ = unix_fs::symlink(gcc, &stub);
            println!("cargo:rustc-link-search=native={}", out_dir);
        }
    }
}
