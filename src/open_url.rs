use std::ffi::OsStr;
use std::io;

#[cfg(target_os = "macos")]
pub fn open(target: impl AsRef<OsStr>) -> io::Result<()> {
    use objc2_app_kit::NSWorkspace;
    use objc2_foundation::{NSString, NSURL};

    let target = target.as_ref();

    if let Some(target_str) = target.to_str() {
        let target_string = NSString::from_str(target_str);
        let url = if url::Url::parse(target_str).is_ok() {
            NSURL::URLWithString(&target_string)
        } else {
            Some(NSURL::fileURLWithPath(&target_string))
        };

        if let Some(url) = url
            && NSWorkspace::sharedWorkspace().openURL(&url)
        {
            return Ok(());
        }
    }

    open::that_detached(target)
}

#[cfg(not(target_os = "macos"))]
pub fn open(target: impl AsRef<OsStr>) -> io::Result<()> {
    open::that_detached(target)
}
