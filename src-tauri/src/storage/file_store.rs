use keyring_core::api::{
	CredentialApi, CredentialPersistence, CredentialStoreApi,
};
use keyring_core::{Credential, CredentialStore, Entry, Error, Result};
use std::any::Any;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

fn credential_path(
	base: &std::path::Path,
	_service: &str,
	user: &str,
) -> PathBuf {
	let safe = |s: &str| s.replace(['/', '\\', '\0', ':'], "_");
	base.join("credentials").join(safe(user))
}

fn temp_path(path: &std::path::Path) -> PathBuf {
	static NEXT: AtomicU64 = AtomicU64::new(0);
	let mut temp = path.to_path_buf().into_os_string();
	temp.push(format!(
		".{}.{}.tmp",
		std::process::id(),
		NEXT.fetch_add(1, Ordering::Relaxed)
	));
	PathBuf::from(temp)
}

fn write_private(path: &std::path::Path, secret: &[u8]) -> std::io::Result<()> {
	use std::io::Write;

	let mut opts = fs::OpenOptions::new();
	opts.write(true).create_new(true);

	#[cfg(unix)]
	{
		use std::os::unix::fs::OpenOptionsExt;
		opts.mode(0o600);
	}

	let mut file = opts.open(path)?;
	file.write_all(secret)?;
	file.sync_all()
}

#[derive(Debug, Clone)]
pub struct FileCredential {
	service: String,
	user: String,
	base: PathBuf,
}

impl CredentialApi for FileCredential {
	fn set_secret(&self, secret: &[u8]) -> Result<()> {
		let path = credential_path(&self.base, &self.service, &self.user);
		let dir = path.parent().ok_or_else(|| {
			Error::PlatformFailure(Box::new(std::io::Error::new(
				std::io::ErrorKind::InvalidInput,
				"credential path has no parent directory",
			)))
		})?;
		fs::create_dir_all(dir)
			.map_err(|e| Error::PlatformFailure(Box::new(e)))?;

		#[cfg(unix)]
		{
			use std::os::unix::fs::PermissionsExt;
			fs::set_permissions(dir, fs::Permissions::from_mode(0o700))
				.map_err(|e| Error::PlatformFailure(Box::new(e)))?;
		}

		let temp = temp_path(&path);
		write_private(&temp, secret)
			.and_then(|()| fs::rename(&temp, &path))
			.map_err(|e| {
				fs::remove_file(&temp).ok();
				Error::PlatformFailure(Box::new(e))
			})
	}

	fn get_secret(&self) -> Result<Vec<u8>> {
		let path = credential_path(&self.base, &self.service, &self.user);
		match fs::read(&path) {
			Ok(bytes) => Ok(bytes),
			Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
				Err(Error::NoEntry)
			}
			Err(e) => Err(Error::PlatformFailure(Box::new(e))),
		}
	}

	fn delete_credential(&self) -> Result<()> {
		let path = credential_path(&self.base, &self.service, &self.user);
		match fs::remove_file(&path) {
			Ok(()) => Ok(()),
			Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
				Err(Error::NoEntry)
			}
			Err(e) => Err(Error::PlatformFailure(Box::new(e))),
		}
	}

	fn get_credential(&self) -> Result<Option<Arc<Credential>>> {
		Ok(None)
	}

	fn get_specifiers(&self) -> Option<(String, String)> {
		Some((self.service.clone(), self.user.clone()))
	}

	fn as_any(&self) -> &dyn Any {
		self
	}
}

#[derive(Debug)]
pub struct FileStore {
	base: PathBuf,
}

impl CredentialStoreApi for FileStore {
	fn vendor(&self) -> String {
		"open-grind file store".to_string()
	}

	fn id(&self) -> String {
		"file-store-v1".to_string()
	}

	fn build(
		&self,
		service: &str,
		user: &str,
		_modifiers: Option<&HashMap<&str, &str>>,
	) -> Result<Entry> {
		let cred = Arc::new(FileCredential {
			service: service.to_string(),
			user: user.to_string(),
			base: self.base.clone(),
		}) as Arc<Credential>;
		Ok(Entry::new_with_credential(cred))
	}

	fn persistence(&self) -> CredentialPersistence {
		CredentialPersistence::UntilDelete
	}

	fn as_any(&self) -> &dyn Any {
		self
	}
}

pub fn init(base: PathBuf) {
	let creds_dir = base.join("credentials");
	if let Ok(()) = fs::create_dir_all(&creds_dir) {
		#[cfg(unix)]
		{
			use std::os::unix::fs::PermissionsExt;
			let _ = fs::set_permissions(
				&creds_dir,
				fs::Permissions::from_mode(0o700),
			);
		}
	}
	let store = Arc::new(FileStore { base }) as Arc<CredentialStore>;
	keyring_core::set_default_store(store);
}
