use windows::{
    // core::*,
    // Win32::{Foundation::*, Security::Credentials::*, UI::WindowsAndMessaging::*},
};

fn main() -> Result<()> {
    unsafe {
        // Initialize credential UI info structure
        let mut ui_info = CREDUI_INFOW {
            cbSize: std::mem::size_of::<CREDUI_INFOW>() as u32,
            hwndParent: HWND::default(),
            pszMessageText: w!("Please enter your credentials"),
            pszCaptionText: w!("Authentication Required"),
            hbmBanner: HBITMAP::default(),
        };

        // Buffer for the authentication package
        let mut auth_package: u32 = 0;
        let mut out_credential_buffer: *mut std::ffi::c_void = std::ptr::null_mut();
        let mut out_credential_buffer_size: u32 = 0;
        let mut save_credentials = false;

        // Flags for the credential prompt
        let flags = CREDUIWIN_GENERIC | CREDUIWIN_ENUMERATE_CURRENT_USER;

        // Call the credential prompt function
        let result = CredUIPromptForWindowsCredentialsW(
            &ui_info,
            0, // dwAuthError - 0 means no previous auth error
            &mut auth_package,
            std::ptr::null(), // pvInAuthBuffer - no input credentials
            0,                // ulInAuthBufferSize
            &mut out_credential_buffer,
            &mut out_credential_buffer_size,
            Some(&mut save_credentials),
            flags,
        );

        match result {
            ERROR_SUCCESS => {
                println!("Credentials obtained successfully!");
                println!("Auth package: {}", auth_package);
                println!("Buffer size: {}", out_credential_buffer_size);
                println!("Save credentials: {}", save_credentials);

                // Unpack the credentials from the buffer
                let mut username_buffer = [0u16; 256];
                let mut username_size = username_buffer.len() as u32;
                let mut domain_buffer = [0u16; 256];
                let mut domain_size = domain_buffer.len() as u32;
                let mut password_buffer = [0u16; 256];
                let mut password_size = password_buffer.len() as u32;

                let unpack_result = CredUnPackAuthenticationBufferW(
                    CRED_PACK_PROTECTED_CREDENTIALS,
                    out_credential_buffer,
                    out_credential_buffer_size,
                    Some(username_buffer.as_mut_ptr()),
                    &mut username_size,
                    Some(domain_buffer.as_mut_ptr()),
                    &mut domain_size,
                    Some(password_buffer.as_mut_ptr()),
                    &mut password_size,
                );

                if unpack_result.is_ok() {
                    let username =
                        String::from_utf16_lossy(&username_buffer[..username_size as usize - 1]);
                    let domain =
                        String::from_utf16_lossy(&domain_buffer[..domain_size as usize - 1]);

                    println!("Username: {}", username);
                    println!("Domain: {}", domain);
                    // Note: Don't print the password in production code!
                    println!("Password length: {}", password_size - 1);
                } else {
                    println!("Failed to unpack credentials: {:?}", unpack_result);
                }

                // Clean up the credential buffer
                if !out_credential_buffer.is_null() {
                    CoTaskMemFree(Some(out_credential_buffer));
                }
            }
            ERROR_CANCELLED => {
                println!("User cancelled the credential prompt");
            }
            _ => {
                println!("Error occurred: {}", result.0);
            }
        }
    }

    Ok(())
}
