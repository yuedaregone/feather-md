#[cfg(target_os = "windows")]
use winreg::enums::*;
#[cfg(target_os = "windows")]
use winreg::RegKey;

#[allow(dead_code)]
const APP_NAME: &str = "FeatherMD";
#[allow(dead_code)]
const PROG_ID: &str = "FeatherMD.md";

/// Register .md file association in HKEY_CURRENT_USER
#[cfg(target_os = "windows")]
pub fn register_association(exe_path: &str) -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let software_classes = hkcu
        .open_subkey_with_flags("Software\\Classes", KEY_WRITE)
        .map_err(|e| format!("Cannot open Software\\Classes: {}", e))?;

    // Backup existing .md association
    let md_key = software_classes
        .open_subkey_with_flags(".md", KEY_READ)
        .ok();
    if let Some(key) = md_key {
        if let Ok(existing) = key.get_value::<String, _>("") {
            if existing != PROG_ID {
                // Save backup
                let hkcu2 = RegKey::predef(HKEY_CURRENT_USER);
                if let Ok((app_key, _)) =
                    hkcu2.create_subkey(format!("Software\\{}\\Backup", APP_NAME))
                {
                    let _ = app_key.set_value(".md", &existing);
                }
            }
        }
    }

    // Set .md → FeatherMD.md
    let (md_key, _) = software_classes
        .create_subkey(".md")
        .map_err(|e| format!("Cannot create .md key: {}", e))?;
    md_key
        .set_value("", &PROG_ID)
        .map_err(|e| format!("Cannot set .md default: {}", e))?;

    // Create FeatherMD.md progid
    let (prog_key, _) = software_classes
        .create_subkey(PROG_ID)
        .map_err(|e| format!("Cannot create {} key: {}", PROG_ID, e))?;
    prog_key
        .set_value("", &"Markdown File")
        .map_err(|e| e.to_string())?;

    // Set icon
    let (icon_key, _) = prog_key
        .create_subkey("DefaultIcon")
        .map_err(|e| e.to_string())?;
    icon_key
        .set_value("", &format!("{},0", exe_path))
        .map_err(|e| e.to_string())?;

    // Set open command
    let (shell_key, _) = prog_key.create_subkey("shell").map_err(|e| e.to_string())?;
    let (open_key, _) = shell_key.create_subkey("open").map_err(|e| e.to_string())?;
    let (cmd_key, _) = open_key
        .create_subkey("command")
        .map_err(|e| e.to_string())?;
    cmd_key
        .set_value("", &format!("\"{}\" \"%1\"", exe_path))
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Unregister .md file association and restore backup
#[cfg(target_os = "windows")]
pub fn unregister_association() -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let software_classes = hkcu
        .open_subkey_with_flags("Software\\Classes", KEY_WRITE)
        .map_err(|e| format!("Cannot open Software\\Classes: {}", e))?;

    // Restore backup
    let backup_key = hkcu
        .open_subkey_with_flags(format!("Software\\{}\\Backup", APP_NAME), KEY_READ)
        .ok();
    if let Some(key) = backup_key {
        if let Ok(original) = key.get_value::<String, _>(".md") {
            let md_key = software_classes
                .open_subkey_with_flags(".md", KEY_WRITE)
                .ok();
            if let Some(k) = md_key {
                let _ = k.set_value("", &original);
            }
        }
    }

    // Delete our progid
    let _ = software_classes.delete_subkey_all(PROG_ID);

    // Delete backup key
    let _ = hkcu.delete_subkey_all(format!("Software\\{}\\Backup", APP_NAME));

    Ok(())
}

/// Check if file association is registered
#[cfg(target_os = "windows")]
pub fn is_registered() -> bool {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let software_classes = hkcu
        .open_subkey_with_flags("Software\\Classes", KEY_READ)
        .ok();

    if let Some(classes) = software_classes {
        if let Ok(md_key) = classes.open_subkey(".md") {
            if let Ok(default) = md_key.get_value::<String, _>("") {
                return default == PROG_ID;
            }
        }
    }
    false
}

#[cfg(not(target_os = "windows"))]
pub fn register_association(_exe_path: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn unregister_association() -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn is_registered() -> bool {
    false
}
