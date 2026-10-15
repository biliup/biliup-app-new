fn main() {
    tauri_build::build();

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        // Tauri 只给应用二进制嵌入资源；单元测试也需要启用公共控件 v6。
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!(concat!(
            "cargo:rustc-link-arg=/MANIFESTDEPENDENCY:type='win32' ",
            "name='Microsoft.Windows.Common-Controls' version='6.0.0.0' ",
            "processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'"
        ));
    }
}
