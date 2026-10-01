//! Dragging shelf files out to other apps, via the shell's own drag and drop: the shell
//! builds the data object (so Explorer, browsers and chat apps accept it) and the drag image.

use std::path::PathBuf;

use windows::{
    core::HSTRING,
    Win32::{
        Foundation::HWND,
        System::{
            Com::IDataObject,
            Ole::{IDropSource, DROPEFFECT_COPY, DROPEFFECT_LINK},
        },
        UI::Shell::{
            BHID_DataObject, ILCreateFromPathW, ILFree, SHCreateShellItemArrayFromIDLists, SHDoDragDrop,
        },
    },
};

use crate::error::{AppError, AppResult};

/// Start a drag of `paths` from `hwnd`. Must run on the UI thread while the mouse button is
/// held; returns once the files are dropped (true) or the drag is cancelled (false).
pub fn drag_files(hwnd: HWND, paths: &[PathBuf]) -> AppResult<bool> {
    // SAFETY: each ID list is created from a path and freed below after the shell item array
    // (which copies them) is built; the COM objects are reference-counted by the bindings.
    unsafe {
        let pidls: Vec<_> = paths
            .iter()
            .map(|p| ILCreateFromPathW(&HSTRING::from(p.as_os_str())))
            .filter(|p| !p.is_null())
            .collect();
        if pidls.is_empty() {
            return Err(AppError::InvalidInput("Those files were moved or deleted".into()));
        }
        let consts: Vec<_> = pidls.iter().map(|p| p.cast_const()).collect();
        let items = SHCreateShellItemArrayFromIDLists(&consts);
        for pidl in pidls {
            ILFree(Some(pidl.cast_const()));
        }
        let data: IDataObject = items?.BindToHandler(None, &BHID_DataObject)?;
        match SHDoDragDrop(
            Some(hwnd),
            &data,
            None::<&IDropSource>,
            DROPEFFECT_COPY | DROPEFFECT_LINK,
        ) {
            Ok(effect) => Ok(effect.0 != 0),
            // DRAGDROP_S_CANCEL comes back as a success code; anything else is a real failure.
            Err(error) => Err(error.into()),
        }
    }
}

/// Open the file's folder in Explorer with the file selected.
pub fn reveal(path: &std::path::Path) -> AppResult<()> {
    use std::os::windows::process::CommandExt;
    // Paths come from the shelf (validated, no quotes), so quoting them is safe.
    std::process::Command::new("explorer.exe")
        .raw_arg(format!("/select,\"{}\"", path.display()))
        .spawn()?;
    Ok(())
}
