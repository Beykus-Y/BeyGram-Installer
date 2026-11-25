use super::{Bundle, PlistInfoTrait};
use crate::{Error, SignerApp, SignerOptions, cgbi};
use plist::Dictionary;
use std::path::{Component, PathBuf};
use std::{fs, io::Read};
use zip::write::FileOptions;
use zip::{ZipArchive, read::ZipFile};

#[derive(Debug, Clone)]
pub struct Package {
    package_file: PathBuf,
    stage_dir: PathBuf,
    info_plist_dictionary: Dictionary,
    archive_entries: Vec<String>,
    pub app_icon_data: Option<Vec<u8>>,
}

impl Package {
    pub fn new(package_file: PathBuf) -> Result<Self, Error> {
        let stage = tempfile::Builder::new().prefix("plume_stage_").tempdir()?;
        let out_package_file = stage.path().join("stage.ipa");
        fs::copy(&package_file, &out_package_file)?;

        let file = fs::File::open(&out_package_file)?;
        let mut archive = ZipArchive::new(file)?;
        let mut archive_entries = Vec::with_capacity(archive.len());
        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            validate_entry(&file)?;
            archive_entries.push(file.name()?.into_owned());
        }

        let info_plist_dictionary =
            Self::get_info_plist_from_archive(&out_package_file, &archive_entries)?;

        let app_icon_data = Self::extract_icon_from_archive(
            &out_package_file,
            &archive_entries,
            &info_plist_dictionary,
        );

        let stage_dir = stage.keep();
        Ok(Self {
            package_file: out_package_file,
            stage_dir: stage_dir.clone(),
            info_plist_dictionary,
            archive_entries,
            app_icon_data,
        })
    }

    pub fn package_file(&self) -> &PathBuf {
        &self.package_file
    }

    fn get_info_plist_from_archive(
        archive_path: &PathBuf,
        archive_entries: &[String],
    ) -> Result<Dictionary, Error> {
        let file = fs::File::open(archive_path)?;
        let mut archive = ZipArchive::new(file)?;

        let info_plist_path = archive_entries
            .iter()
            .find(|entry| {
                entry.starts_with("Payload/")
                    && entry.ends_with("/Info.plist")
                    && entry.matches('/').count() == 2
            })
            .ok_or(Error::PackageInfoPlistMissing)?;

        let plist_data = read_entry(archive.by_name(info_plist_path)?, 8 * 1024 * 1024)?;

        Ok(plist::from_bytes(&plist_data)?)
    }

    fn extract_icon_from_archive(
        archive_path: &PathBuf,
        archive_entries: &[String],
        plist: &Dictionary,
    ) -> Option<Vec<u8>> {
        // Collects all candidate icon base names from the plist, in order of preference.
        // CFBundleIcons (iPhone) takes priority, fall back to CFBundleIcons~ipad, then
        // top-level CFBundleIconFiles.
        let mut icon_names: Vec<String> = Vec::new();

        let primary_from = |d: &Dictionary| -> Vec<String> {
            d.get("CFBundlePrimaryIcon")
                .and_then(|v| v.as_dictionary())
                .and_then(|d| d.get("CFBundleIconFiles"))
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_string())
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        };

        if let Some(d) = plist.get("CFBundleIcons").and_then(|v| v.as_dictionary()) {
            icon_names.extend(primary_from(d));
        }
        if let Some(d) = plist
            .get("CFBundleIcons~ipad")
            .and_then(|v| v.as_dictionary())
        {
            for n in primary_from(d) {
                if !icon_names.contains(&n) {
                    icon_names.push(n);
                }
            }
        }
        if let Some(arr) = plist.get("CFBundleIconFiles").and_then(|v| v.as_array()) {
            for n in arr
                .iter()
                .filter_map(|v| v.as_string())
                .map(|s| s.to_string())
            {
                if !icon_names.contains(&n) {
                    icon_names.push(n);
                }
            }
        }

        if icon_names.is_empty() {
            return None;
        }

        let app_prefix = archive_entries
            .iter()
            .find(|e| {
                e.starts_with("Payload/")
                    && e.ends_with("/Info.plist")
                    && e.matches('/').count() == 2
            })?
            .trim_end_matches("/Info.plist");

        let file = fs::File::open(archive_path).ok()?;
        let mut archive = ZipArchive::new(file).ok()?;

        let suffixes = ["@3x.png", "@2x.png", "@1x.png", ".png"];

        for name in &icon_names {
            for suffix in &suffixes {
                let candidate = format!("{app_prefix}/{name}{suffix}");
                if let Ok(entry) = archive.by_name(&candidate)
                    && let Ok(data) = read_entry(entry, 32 * 1024 * 1024)
                    && !data.is_empty()
                {
                    return Some(cgbi::normalize(data));
                }
            }
        }

        None
    }

    pub fn get_package_bundle(&self) -> Result<Bundle, Error> {
        let file = fs::File::open(&self.package_file)?;
        let mut archive = ZipArchive::new(file)?;
        let extraction_dir = tempfile::Builder::new()
            .prefix("unpacked_")
            .tempdir_in(&self.stage_dir)?;
        archive.extract(extraction_dir.path())?;

        let app_dir = fs::read_dir(extraction_dir.path().join("Payload"))?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .find(|p| p.is_dir() && p.extension().and_then(|e| e.to_str()) == Some("app"))
            .ok_or_else(|| Error::PackageInfoPlistMissing)?;

        let bundle = Bundle::new(app_dir)?;
        extraction_dir.keep();
        Ok(bundle)
    }

    pub fn get_archive_based_on_path(&self, path: &PathBuf) -> Result<PathBuf, Error> {
        if path.is_dir() {
            let payload_dir = path.parent().ok_or(Error::PackageInfoPlistMissing)?;
            self.archive_package_bundle(payload_dir)
        } else {
            Ok(self.package_file.clone())
        }
    }

    fn archive_package_bundle(&self, payload_dir: &std::path::Path) -> Result<PathBuf, Error> {
        let prefix = payload_dir.parent().ok_or(Error::PackageInfoPlistMissing)?;
        let file = tempfile::Builder::new()
            .prefix("resigned_")
            .suffix(".ipa")
            .tempfile_in(&self.stage_dir)?;
        let mut zip = zip::ZipWriter::new(file);
        let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        fn add_dir_to_zip(
            zip: &mut zip::ZipWriter<tempfile::NamedTempFile>,
            path: &std::path::Path,
            prefix: &std::path::Path,
            options: &FileOptions<'_, zip::write::ExtendedFileOptions>,
        ) -> Result<(), Error> {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let entry_path = entry.path();
                let name = entry_path
                    .strip_prefix(prefix)
                    .map_err(|_| Error::PackageInfoPlistMissing)?
                    .to_string_lossy()
                    .replace('\\', "/");

                if entry_path.is_file() {
                    zip.start_file(&name, options.clone())?;
                    let mut f = fs::File::open(&entry_path)?;
                    std::io::copy(&mut f, zip)?;
                } else if entry_path.is_dir() {
                    zip.add_directory(&name, options.clone())?;
                    add_dir_to_zip(zip, &entry_path, prefix, options)?;
                }
            }
            Ok(())
        }

        add_dir_to_zip(&mut zip, payload_dir, prefix, &options)?;
        let (_, zip_file_path) = zip.finish()?.keep().map_err(|err| err.error)?;
        Ok(zip_file_path)
    }

    pub fn remove_package_stage(self) {
        fs::remove_dir_all(&self.stage_dir).ok();
    }
}

// TODO: make bundle and package share a common trait for plist info access
macro_rules! get_plist_dict_value {
    ($self:ident, $key:expr) => {{
        $self
            .info_plist_dictionary
            .get($key)
            .and_then(|v| v.as_string())
            .map(|s| s.to_string())
    }};
}

impl PlistInfoTrait for Package {
    fn get_name(&self) -> Option<String> {
        get_plist_dict_value!(self, "CFBundleDisplayName")
            .or_else(|| get_plist_dict_value!(self, "CFBundleName"))
            .or_else(|| self.get_executable())
    }

    fn get_executable(&self) -> Option<String> {
        get_plist_dict_value!(self, "CFBundleExecutable")
    }

    fn get_bundle_identifier(&self) -> Option<String> {
        get_plist_dict_value!(self, "CFBundleIdentifier")
    }

    fn get_bundle_name(&self) -> Option<String> {
        get_plist_dict_value!(self, "CFBundleName")
    }

    fn get_version(&self) -> Option<String> {
        get_plist_dict_value!(self, "CFBundleShortVersionString")
    }

    fn get_build_version(&self) -> Option<String> {
        get_plist_dict_value!(self, "CFBundleVersion")
    }
}

impl Package {
    pub fn load_into_signer_options<'settings, 'slf: 'settings>(
        &'slf self,
        settings: &'settings mut SignerOptions,
    ) {
        let app = if self
            .archive_entries
            .iter()
            .any(|entry| entry.contains("SideStoreApp.framework"))
        {
            SignerApp::LiveContainerAndSideStore
        } else {
            SignerApp::from_bundle_identifier(self.get_bundle_identifier().as_deref())
        };

        let new_settings = SignerOptions::new_for_app(app);
        *settings = new_settings;
    }
}

fn validate_entry<R: Read>(entry: &ZipFile<'_, R>) -> Result<(), Error> {
    let valid_path = entry.enclosed_name().is_some_and(|path| {
        path.components()
            .all(|part| matches!(part, Component::Normal(_)))
    }) && !entry.name()?.contains(['\\', ':']);
    let regular = entry
        .unix_mode()
        .is_none_or(|mode| matches!(mode & 0o170000, 0 | 0o100000 | 0o040000));
    if !valid_path || !regular || entry.is_symlink() {
        return Err(Error::UnsafePackageEntry);
    }
    Ok(())
}

fn read_entry(entry: impl Read, limit: u64) -> Result<Vec<u8>, Error> {
    let mut content = Vec::new();
    entry.take(limit + 1).read_to_end(&mut content)?;
    if content.len() as u64 > limit {
        return Err(Error::PackageMetadataTooLarge);
    }
    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_size_limit_applies_to_actual_decoded_bytes() {
        assert_eq!(read_entry(&b"1234"[..], 4).unwrap(), b"1234");
        assert!(matches!(
            read_entry(&b"12345"[..], 4),
            Err(Error::PackageMetadataTooLarge)
        ));
    }
}
