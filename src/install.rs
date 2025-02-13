// Jackson Coxson

use log::{error, info};
use rusty_libimobiledevice::services::{afc::AfcFileMode, instproxy::InstProxyClient};

use crate::{device::fetch_first_device, test_device_connection, Errors, Result};

const PKG_PATH: &str = "PublicStaging";

/// Yeets an ipa to the afc jail
pub fn yeet_app_afc(bundle_id: String, ipa_bytes: &[u8]) -> Result<()> {
    info!("Yeeting IPA for bundle ID: {}", bundle_id);

    if !test_device_connection() {
        error!("No device connection");
        return Err(Errors::NoConnection);
    }

    let device = match fetch_first_device() {
        Some(d) => d,
        None => return Err(Errors::NoDevice),
    };

    // Start an AFC client
    let afc = match device.new_afc_client("minimuxer") {
        Ok(afc) => afc,
        Err(e) => {
            error!("Error: Could not start AFC service: {:?}", e);
            return Err(Errors::CreateAfc);
        }
    };

    // Check if PublicStaging exists
    match afc.get_file_info(format!("./{PKG_PATH}")) {
        Ok(_) => {}
        Err(_) => match afc.make_directory(format!("./{PKG_PATH}")) {
            Ok(_) => match afc.get_file_info(format!("./{PKG_PATH}")) {
                Ok(_) => {}
                Err(e) => {
                    error!("Unable to read PublicStaging info: {:?}", e);
                    return Err(Errors::RwAfc);
                }
            },
            Err(e) => {
                error!("Unable to make PublicStaging directory: {:?}", e);
                return Err(Errors::RwAfc);
            }
        },
    };
    info!("Created PublicStaging directory");

    // Create bundle ID folder
    match afc.get_file_info(format!("./{PKG_PATH}/{bundle_id}")) {
        Ok(_) => {}
        Err(_) => match afc.make_directory(format!("./{PKG_PATH}/{bundle_id}")) {
            Ok(_) => match afc.get_file_info(format!("./{PKG_PATH}/{bundle_id}")) {
                Ok(_) => {}
                Err(e) => {
                    error!("Unable to read bundle ID info: {:?}", e);
                    return Err(Errors::RwAfc);
                }
            },
            Err(e) => {
                error!("Unable to make bundle ID directory: {:?}", e);
                return Err(Errors::RwAfc);
            }
        },
    };
    info!("Created bundle ID directory");

    // Yeet app pls
    let handle = match afc.file_open(
        format!("./{PKG_PATH}/{bundle_id}/app.ipa"),
        AfcFileMode::WriteOnly,
    ) {
        Ok(h) => h,
        Err(e) => {
            error!("Unable to open file on device: {:?}", e);
            return Err(Errors::RwAfc);
        }
    };

    info!("Sending bytes of ipa");
    match afc.file_write(handle, ipa_bytes.to_vec()) {
        Ok(_) => {
            info!("Success");
            Ok(())
        }
        Err(e) => {
            error!("Unable to write ipa: {:?}", e);
            Err(Errors::RwAfc)
        }
    }
}

/// Installs an ipa with a bundle ID
/// Expects the ipa to be in the afc jail from yeet_app_afc
pub fn install_ipa(bundle_id: String) -> Result<()> {
    info!("Installing app for bundle ID: {}", bundle_id);

    if !test_device_connection() {
        error!("No device connection");
        return Err(Errors::NoConnection);
    }

    let device = match fetch_first_device() {
        Some(d) => d,
        None => return Err(Errors::NoDevice),
    };

    let mut client_opts = InstProxyClient::client_options_new();
    client_opts
        .dict_set_item("CFBundleIdentifier", bundle_id.clone().into())
        .unwrap();

    let inst_client = match device.new_instproxy_client("ideviceinstaller") {
        Ok(i) => i,
        Err(e) => {
            error!("Unable to start instproxy: {:?}", e);
            return Err(Errors::CreateInstproxy);
        }
    };

    info!("Installing");
    match inst_client.install(
        format!("./{PKG_PATH}/{bundle_id}/app.ipa"),
        Some(client_opts.clone()), // nobody understands libplist, but clone is necessary I guess
    ) {
        Ok(_) => {
            info!("Done!");
            Ok(())
        }
        Err(e) => {
            error!("Unable to install app: {:?}", e);
            Err(Errors::InstallApp)
        }
    }
}

/// Removes an app from the device
pub fn remove_app(bundle_id: String) -> Result<()> {
    info!("Removing app for {}", bundle_id);

    if !test_device_connection() {
        error!("No device connection");
        return Err(Errors::NoConnection);
    }

    let device = match fetch_first_device() {
        Some(d) => d,
        None => return Err(Errors::NoDevice),
    };

    let instproxy_client = match device.new_instproxy_client("minimuxer-remove-app") {
        Ok(i) => i,
        Err(e) => {
            error!("Unable to start instproxy: {:?}", e);
            return Err(Errors::CreateInstproxy);
        }
    };

    info!("Removing");
    match instproxy_client.uninstall(bundle_id, None) {
        Ok(_) => {
            info!("Done!");
            Ok(())
        }
        Err(e) => {
            error!("Unable to uninstall app!! {:?}", e);
            Err(Errors::UninstallApp)
        }
    }
}
