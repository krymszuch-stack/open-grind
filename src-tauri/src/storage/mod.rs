mod entries;
mod file_store;

pub use entries::{AuthStorage, DeviceStorage, SigningKeyStorage};

pub fn init_file_store(base: std::path::PathBuf) {
	file_store::init(base);
}

pub fn init_keyring() {
	#[cfg(target_os = "ios")]
	match apple_native_keyring_store::protected::Store::new() {
		Ok(store) => keyring_core::set_default_store(store),
		Err(e) => tracing::error!("[storage] could not init iOS keyring: {e}"),
	}

	#[cfg(target_os = "android")]
	match android_native_keyring_store::Store::new() {
		Ok(store) => keyring_core::set_default_store(store),
		Err(e) => {
			tracing::error!("[storage] could not init Android keyring: {e}")
		}
	}

	#[cfg(all(target_os = "macos", feature = "keychain"))]
	match apple_native_keyring_store::keychain::Store::new() {
		Ok(store) => keyring_core::set_default_store(store),
		Err(e) => {
			tracing::error!("[storage] could not init macOS keyring: {e}")
		}
	}
}
