fn main() {
    tauri_build::build();

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        // 为测试生成公共控件 v6 的外部 manifest，避免重复嵌入 Tauri 的应用资源。
        println!("cargo:rustc-link-arg=/MANIFEST");
        println!(concat!(
            "cargo:rustc-link-arg=/MANIFESTDEPENDENCY:type='win32' ",
            "name='Microsoft.Windows.Common-Controls' version='6.0.0.0' ",
            "processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'"
        ));
    }
}
