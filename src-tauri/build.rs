fn main() {
    #[cfg(target_os = "macos")]
    {
        cc::Build::new()
            .file("src/ocr/macos_vision.m")
            .flag("-fmodules")
            .flag("-fobjc-arc")
            .compile("macos_vision");

        println!("cargo:rustc-link-lib=framework=Vision");
        println!("cargo:rustc-link-lib=framework=CoreGraphics");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
        println!("cargo:rustc-link-lib=framework=ImageIO");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=objc");
    }
    tauri_build::build();
}
