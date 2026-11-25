//! Atomic, authenticated storage for records whose size can exceed OS vault limits.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::Path,
};

use ring::{
    aead::{self, Aad, LessSafeKey, Nonce, UnboundKey},
    digest,
    rand::{SecureRandom, SystemRandom},
};
use zeroize::Zeroizing;

const MAGIC: &[u8] = b"BEYGRAM-SECRET\x00\x01";
const KEY_LEN: usize = 32;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("OS credential storage is unavailable or locked")]
    VaultUnavailable,
    #[error("The key for this encrypted record is missing from OS credential storage")]
    MissingKey,
    #[error("Secret record failed authentication")]
    Authentication,
    #[error("Secret record changed in another process. Reload it and retry.")]
    Conflict,
    #[error("Could not generate encryption material")]
    Random,
    #[error("Could not access secret file: {0}")]
    Io(#[from] io::Error),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Revision([u8; 32]);

impl Revision {
    fn for_ciphertext(ciphertext: &[u8]) -> Self {
        Self(
            digest::digest(&digest::SHA256, ciphertext)
                .as_ref()
                .try_into()
                .unwrap(),
        )
    }
}

/// Read a record, replacing valid legacy plaintext in place after it has been encrypted.
/// The validation callback runs before any legacy data is migrated.
pub fn read_or_migrate<T, E>(
    path: &Path,
    decode: impl FnOnce(&[u8]) -> Result<T, E>,
) -> Result<Option<T>, E>
where
    E: From<Error>,
{
    Ok(read_or_migrate_with_revision(path, decode)?.map(|(value, _)| value))
}

pub fn read_or_migrate_with_revision<T, E>(
    path: &Path,
    decode: impl FnOnce(&[u8]) -> Result<T, E>,
) -> Result<Option<(T, Revision)>, E>
where
    E: From<Error>,
{
    let Some(locked) = LockedFile::open(path, false)? else {
        return Ok(None);
    };
    read_locked(&locked, |create| get_key(&locked.entry()?, create), decode)
}

fn read_locked<T, E>(
    locked: &LockedFile<'_>,
    key: impl FnOnce(bool) -> Result<LessSafeKey, Error>,
    decode: impl FnOnce(&[u8]) -> Result<T, E>,
) -> Result<Option<(T, Revision)>, E>
where
    E: From<Error>,
{
    let bytes = match fs::read(locked.path) {
        Ok(bytes) => Zeroizing::new(bytes),
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(Error::Io(err).into()),
    };
    if let Some(ciphertext) = bytes.strip_prefix(MAGIC) {
        let plaintext = decrypt(&key(false)?, &locked.id, ciphertext)?;
        Ok(Some((
            decode(&plaintext)?,
            Revision::for_ciphertext(ciphertext),
        )))
    } else {
        let decoded = decode(&bytes)?;
        let revision = locked.save(&key(true)?, &bytes)?;
        Ok(Some((decoded, revision)))
    }
}

/// Ensure a vault key is available before starting an external provisioning operation.
pub fn prepare(path: &Path) -> Result<(), Error> {
    let locked = LockedFile::open(path, true)?.ok_or(Error::Authentication)?;
    get_key(&locked.entry()?, !path.try_exists()?)?;
    Ok(())
}

pub fn write(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    write_checked(path, bytes, None).map(|_| ())
}

/// Replace a record only if its current revision matches the one the caller read.
pub fn write_if_current(
    path: &Path,
    bytes: &[u8],
    revision: Option<Revision>,
) -> Result<Revision, Error> {
    write_checked(path, bytes, Some(revision))
}

fn write_checked(
    path: &Path,
    bytes: &[u8],
    expected: Option<Option<Revision>>,
) -> Result<Revision, Error> {
    let locked = LockedFile::open(path, true)?.ok_or(Error::Authentication)?;
    write_locked(&locked, bytes, expected, |create| {
        get_key(&locked.entry()?, create)
    })
}

fn write_locked(
    locked: &LockedFile<'_>,
    bytes: &[u8],
    expected: Option<Option<Revision>>,
    key: impl FnOnce(bool) -> Result<LessSafeKey, Error>,
) -> Result<Revision, Error> {
    match fs::read(locked.path) {
        Ok(existing) => {
            let existing = Zeroizing::new(existing);
            if let Some(ciphertext) = existing.strip_prefix(MAGIC) {
                if expected.is_some_and(|value| value != Some(Revision::for_ciphertext(ciphertext)))
                {
                    return Err(Error::Conflict);
                }
                let key = key(false)?;
                decrypt(&key, &locked.id, ciphertext)?;
                return locked.save(&key, bytes);
            }
            if expected.is_some() {
                return Err(Error::Conflict);
            }
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            if expected.flatten().is_some() {
                return Err(Error::Conflict);
            }
        }
        Err(err) => return Err(err.into()),
    }
    locked.save(&key(true)?, bytes)
}

struct LockedFile<'a> {
    path: &'a Path,
    id: String,
    _lock: File,
}

impl<'a> LockedFile<'a> {
    fn open(path: &'a Path, create: bool) -> Result<Option<Self>, Error> {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let name = path
            .file_name()
            .ok_or(io::ErrorKind::InvalidInput)
            .map_err(io::Error::from)?;
        if create {
            fs::create_dir_all(parent)?;
        } else if !path.try_exists()? {
            return Ok(None);
        }
        let canonical = parent.canonicalize()?.join(name);
        let id = hex::encode(digest::digest(
            &digest::SHA256,
            canonical.as_os_str().as_encoded_bytes(),
        ));
        let mut lock_name = name.to_os_string();
        lock_name.push(".lock");
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(parent.join(lock_name))?;
        lock.lock()?;
        match path.symlink_metadata() {
            Ok(metadata) if !metadata.is_file() => {
                return Err(io::Error::from(io::ErrorKind::InvalidInput).into());
            }
            Ok(_) => {}
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(err.into()),
        }
        Ok(Some(Self {
            path,
            id,
            _lock: lock,
        }))
    }

    fn entry(&self) -> Result<keyring::Entry, Error> {
        keyring::Entry::new("BeyGram Installer", &self.id).map_err(|_| Error::VaultUnavailable)
    }

    fn save(&self, key: &LessSafeKey, bytes: &[u8]) -> Result<Revision, Error> {
        let ciphertext = encrypt(key, &self.id, bytes)?;
        let parent = self
            .path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let mut temp = tempfile::NamedTempFile::new_in(parent)?;
        temp.write_all(MAGIC)?;
        temp.write_all(&ciphertext)?;
        temp.as_file().sync_all()?;
        temp.persist(self.path).map_err(|err| err.error)?;
        Ok(Revision::for_ciphertext(&ciphertext))
    }
}

fn get_key(entry: &keyring::Entry, create: bool) -> Result<LessSafeKey, Error> {
    let key = match entry.get_secret() {
        Ok(key) => Zeroizing::new(key),
        Err(keyring::Error::NoEntry) if create => {
            let mut key = Zeroizing::new(vec![0; KEY_LEN]);
            SystemRandom::new()
                .fill(&mut key)
                .map_err(|_| Error::Random)?;
            entry
                .set_secret(&key)
                .map_err(|_| Error::VaultUnavailable)?;
            let saved = Zeroizing::new(entry.get_secret().map_err(|_| Error::VaultUnavailable)?);
            if *saved != *key {
                return Err(Error::VaultUnavailable);
            }
            key
        }
        Err(keyring::Error::NoEntry) => return Err(Error::MissingKey),
        Err(_) => return Err(Error::VaultUnavailable),
    };
    let key = UnboundKey::new(&aead::CHACHA20_POLY1305, &key).map_err(|_| Error::Authentication)?;
    Ok(LessSafeKey::new(key))
}

fn encrypt(key: &LessSafeKey, id: &str, plaintext: &[u8]) -> Result<Vec<u8>, Error> {
    let mut nonce = [0; aead::NONCE_LEN];
    SystemRandom::new()
        .fill(&mut nonce)
        .map_err(|_| Error::Random)?;
    let mut ciphertext = Zeroizing::new(plaintext.to_vec());
    key.seal_in_place_append_tag(
        Nonce::assume_unique_for_key(nonce),
        Aad::from(id),
        &mut *ciphertext,
    )
    .map_err(|_| Error::Authentication)?;
    let mut record = nonce.to_vec();
    record.extend_from_slice(&ciphertext);
    Ok(record)
}

fn decrypt(key: &LessSafeKey, id: &str, record: &[u8]) -> Result<Zeroizing<Vec<u8>>, Error> {
    let (nonce, ciphertext) = record
        .split_at_checked(aead::NONCE_LEN)
        .ok_or(Error::Authentication)?;
    let nonce = Nonce::try_assume_unique_for_key(nonce).map_err(|_| Error::Authentication)?;
    let mut plaintext = Zeroizing::new(ciphertext.to_vec());
    let n = key
        .open_in_place(nonce, Aad::from(id), &mut plaintext)
        .map_err(|_| Error::Authentication)?
        .len();
    plaintext.truncate(n);
    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_entry() -> keyring::Entry {
        keyring::Entry::new_with_credential(Box::new(keyring::mock::MockCredential::default()))
    }

    #[test]
    fn migrates_legacy_record_and_rejects_tampered_ciphertext() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("fixture");
        let marker = b"synthetic secret";
        fs::write(&path, marker).unwrap();
        let locked = LockedFile::open(&path, false).unwrap().unwrap();
        let entry = mock_entry();
        let decode = |bytes: &[u8]| Ok::<_, Error>(bytes.to_vec());

        let migrated = read_locked(&locked, |create| get_key(&entry, create), decode).unwrap();
        assert_eq!(
            migrated.as_ref().map(|(bytes, _)| bytes.as_slice()),
            Some(marker.as_slice())
        );
        let mut file = fs::read(&path).unwrap();
        assert!(file.starts_with(MAGIC));
        assert!(!file.windows(marker.len()).any(|part| part == marker));

        let reopened = read_locked(&locked, |create| get_key(&entry, create), decode).unwrap();
        assert_eq!(
            reopened.as_ref().map(|(bytes, _)| bytes.as_slice()),
            Some(marker.as_slice())
        );
        assert!(matches!(
            read_locked(&locked, |create| get_key(&mock_entry(), create), decode),
            Err(Error::MissingKey)
        ));

        *file.last_mut().unwrap() ^= 1;
        fs::write(&path, file).unwrap();
        assert!(matches!(
            read_locked(&locked, |create| get_key(&entry, create), decode),
            Err(Error::Authentication)
        ));
    }

    #[test]
    fn a_stale_snapshot_cannot_restore_removed_credentials() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("fixture");
        let locked = LockedFile::open(&path, true).unwrap().unwrap();
        let entry = mock_entry();
        let key = |create| get_key(&entry, create);
        let original = write_locked(&locked, b"synthetic token", Some(None), key).unwrap();
        write_locked(&locked, b"empty", Some(Some(original)), key).unwrap();

        assert!(matches!(
            write_locked(&locked, b"synthetic token", Some(Some(original)), key),
            Err(Error::Conflict)
        ));
    }

    #[test]
    fn unavailable_vault_keeps_legacy_record_for_retry() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("fixture");
        fs::write(&path, b"synthetic secret").unwrap();
        let locked = LockedFile::open(&path, false).unwrap().unwrap();

        let result = read_locked(
            &locked,
            |_| Err(Error::VaultUnavailable),
            |_: &[u8]| Ok::<_, Error>(()),
        );
        assert!(matches!(result, Err(Error::VaultUnavailable)));
        assert_eq!(fs::read(&path).unwrap(), b"synthetic secret");
    }
}
