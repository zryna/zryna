use std::{fs, path::Path};

use cap_std::{ambient_authority, fs::Dir};
use same_file::Handle;
use zryna_diagnostics::Diagnostic;

use super::{ArtifactOutputRoot, invalid_output_root_error};

impl ArtifactOutputRoot {
    pub(crate) fn retained_directory(&self) -> Result<Dir, Diagnostic> {
        self.retained_directory_with_hook(|| {})
    }

    fn retained_directory_with_hook(&self, before_open: impl FnOnce()) -> Result<Dir, Diagnostic> {
        self.revalidate()?;
        before_open();
        let directory = Dir::open_ambient_dir(&self.path, ambient_authority())
            .map_err(|_| invalid_output_root_error("artifact output root cannot be retained"))?;
        let current =
            directory.try_clone().map(Dir::into_std_file).and_then(Handle::from_file).map_err(
                |_| invalid_output_root_error("artifact output identity cannot be retained"),
            )?;
        if current != *self.identity {
            return Err(invalid_output_root_error(
                "artifact output root changed before capability capture",
            ));
        }
        Ok(directory)
    }

    #[cfg(test)]
    pub(crate) fn retained_directory_with_substitution_hook(
        &self,
        before_open: impl FnOnce(),
    ) -> Result<Dir, Diagnostic> {
        self.retained_directory_with_hook(before_open)
    }
}

pub(crate) fn validate_real_directory_chain(path: &Path) -> Result<(), Diagnostic> {
    for component in path.ancestors() {
        let metadata = fs::symlink_metadata(component).map_err(|error| {
            invalid_output_root_error(format!(
                "could not inspect artifact output path component '{}': {error}",
                component.display()
            ))
        })?;
        if !metadata.is_dir() || metadata_is_link_or_reparse(&metadata) {
            return Err(invalid_output_root_error(format!(
                "artifact output path component '{}' is not a real directory",
                component.display()
            )));
        }
    }
    Ok(())
}

#[cfg(windows)]
pub(super) fn metadata_is_link_or_reparse(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    metadata.file_type().is_symlink()
        || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
pub(super) fn metadata_is_link_or_reparse(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}
