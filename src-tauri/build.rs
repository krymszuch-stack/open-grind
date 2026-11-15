fn main() {
	let is_windows_msvc = std::env::var_os("CARGO_CFG_TARGET_OS").as_deref()
		== Some(std::ffi::OsStr::new("windows"))
		&& std::env::var_os("CARGO_CFG_TARGET_ENV").as_deref()
			== Some(std::ffi::OsStr::new("msvc"));

	if is_windows_msvc {
		// `tauri_build::build()` only passes `resource.lib` via
		// `cargo:rustc-link-arg-bins`, leaving Windows test harnesses without a
		// Common Controls v6 manifest (0xc0000139 on `TaskDialogIndirect`).
		// Let `link.exe` embed the manifest across all MSVC PE targets instead.
		println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
		println!(
			"cargo:rustc-link-arg=/MANIFESTDEPENDENCY:{}",
			concat!(
				"type='win32' ",
				"name='Microsoft.Windows.Common-Controls' ",
				"version='6.0.0.0' ",
				"processorArchitecture='*' ",
				"publicKeyToken='6595b64144ccf1df' ",
				"language='*'",
			)
		);
		let attrs = tauri_build::Attributes::new().windows_attributes(
			tauri_build::WindowsAttributes::new_without_app_manifest(),
		);
		if let Err(error) = tauri_build::try_build(attrs) {
			panic!("error found during tauri-build: {error:#}");
		}
	} else {
		tauri_build::build();
	}
}
