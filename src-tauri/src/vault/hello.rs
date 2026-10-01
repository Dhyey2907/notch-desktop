//! Windows Hello (face, fingerprint or PIN) before revealing or copying a password.

use windows::{
    core::HSTRING,
    Security::Credentials::UI::{
        UserConsentVerificationResult, UserConsentVerifier, UserConsentVerifierAvailability,
    },
    Win32::{Foundation::HWND, System::WinRT::IUserConsentVerifierInterop},
};

use crate::error::{AppError, AppResult};

pub fn available() -> bool {
    UserConsentVerifier::CheckAvailabilityAsync()
        .and_then(|op| op.get())
        .is_ok_and(|a| a == UserConsentVerifierAvailability::Available)
}

/// Ask for Windows Hello, attached to `window` so the prompt appears in front.
pub fn verify(window: isize, message: &str) -> AppResult<()> {
    if !available() {
        return Err(AppError::InvalidInput(
            "Windows Hello isn't set up on this PC. Set it up in Settings → Accounts → Sign-in options, or turn off \"Ask for Windows Hello\".".into(),
        ));
    }
    let interop = windows::core::factory::<UserConsentVerifier, IUserConsentVerifierInterop>()
        .map_err(|e| AppError::Platform(e.to_string()))?;
    // SAFETY: `window` is a live top-level window of this app.
    let operation: windows_future::IAsyncOperation<UserConsentVerificationResult> =
        unsafe { interop.RequestVerificationForWindowAsync(HWND(window as *mut _), &HSTRING::from(message)) }
            .map_err(|e| AppError::Platform(e.to_string()))?;
    match operation.get() {
        Ok(UserConsentVerificationResult::Verified) => Ok(()),
        Ok(UserConsentVerificationResult::Canceled) => Err(AppError::InvalidInput("Cancelled".into())),
        _ => Err(AppError::InvalidInput(
            "Windows Hello didn't confirm it's you".into(),
        )),
    }
}
