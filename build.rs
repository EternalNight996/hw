fn main() {
  println!("cargo:rustc-env=RUSTFLAGS=-C target-cpu=native");
  println!("cargo:rustc-link-arg=-s"); // 剥离所有符号
    #[cfg(feature="build")]
    #[cfg(windows)]
    {
      static_vcruntime::metabuild();
      winresource::WindowsResource::new()
        .set_icon("./scripts/windows/icon.ico")
        .set_manifest(include_str!("./scripts/windows/app.manifest"))
        .set_resource_file("./scripts/windows/app.rc")
        .set_version_info(winresource::VersionInfo::PRODUCTVERSION, 0x0001000000000000)
        .compile()
        .expect("embed Windows resources");
    }
    #[cfg(feature="build")]
    #[cfg(feature = "built")]
    built::write_built_file().expect("Failed to acquire build-time information");
  }
  