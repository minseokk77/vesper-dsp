use serde::{Deserialize, Serialize};
#[cfg(mobile)]
use tauri::plugin::PluginHandle;
#[cfg(desktop)]
use tauri::AppHandle;
use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

#[cfg(mobile)]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OpenUriRequest {
    uri: String,
}

#[cfg(mobile)]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateReceivedFileRequest {
    file_name: String,
}

#[cfg(mobile)]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FinishReceivedFileRequest {
    uri: String,
    success: bool,
}

#[cfg(mobile)]
#[derive(Debug, Deserialize)]
struct DeviceName {
    name: String,
}

#[cfg(mobile)]
#[derive(Debug, Deserialize)]
struct ScannedQr {
    data: String,
}

#[cfg(mobile)]
#[derive(Debug, Deserialize)]
struct SelectedUri {
    uri: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenedContent {
    pub fd: i32,
    pub file_name: String,
    pub file_size: u64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedReceivedFile {
    pub fd: i32,
    pub uri: String,
    pub file_name: String,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[cfg(mobile)]
    #[error(transparent)]
    PluginInvoke(#[from] tauri::plugin::mobile::PluginInvokeError),
    #[error("Android content URI access is unavailable on this platform")]
    Unsupported,
}

type Result<T> = std::result::Result<T, Error>;

pub struct ContentAccess<R: Runtime> {
    #[cfg(mobile)]
    handle: PluginHandle<R>,
    #[cfg(desktop)]
    _app: AppHandle<R>,
}

impl<R: Runtime> ContentAccess<R> {
    pub fn open_uri(&self, uri: String) -> Result<OpenedContent> {
        #[cfg(mobile)]
        {
            return self
                .handle
                .run_mobile_plugin("openUri", OpenUriRequest { uri })
                .map_err(Into::into);
        }
        #[cfg(desktop)]
        {
            let _ = uri;
            Err(Error::Unsupported)
        }
    }

    pub fn create_received_file(&self, file_name: String) -> Result<CreatedReceivedFile> {
        #[cfg(mobile)]
        {
            return self
                .handle
                .run_mobile_plugin(
                    "createReceivedFile",
                    CreateReceivedFileRequest { file_name },
                )
                .map_err(Into::into);
        }
        #[cfg(desktop)]
        {
            let _ = file_name;
            Err(Error::Unsupported)
        }
    }

    pub fn finish_received_file(&self, uri: String, success: bool) -> Result<()> {
        #[cfg(mobile)]
        {
            return self
                .handle
                .run_mobile_plugin(
                    "finishReceivedFile",
                    FinishReceivedFileRequest { uri, success },
                )
                .map_err(Into::into);
        }
        #[cfg(desktop)]
        {
            let _ = (uri, success);
            Err(Error::Unsupported)
        }
    }

    pub fn open_received_folder(&self) -> Result<()> {
        #[cfg(mobile)]
        {
            return self
                .handle
                .run_mobile_plugin("openReceivedFolder", ())
                .map_err(Into::into);
        }
        #[cfg(desktop)]
        {
            Err(Error::Unsupported)
        }
    }

    pub fn request_local_network_access(&self) -> Result<()> {
        #[cfg(mobile)]
        {
            return self
                .handle
                .run_mobile_plugin("requestLocalNetworkAccess", ())
                .map_err(Into::into);
        }
        #[cfg(desktop)]
        {
            Ok(())
        }
    }

    pub fn start_background_service(&self) -> Result<()> {
        #[cfg(mobile)]
        {
            return self
                .handle
                .run_mobile_plugin("startBackgroundService", ())
                .map_err(Into::into);
        }
        #[cfg(desktop)]
        {
            Ok(())
        }
    }

    pub fn stop_background_service(&self) -> Result<()> {
        #[cfg(mobile)]
        {
            return self
                .handle
                .run_mobile_plugin("stopBackgroundService", ())
                .map_err(Into::into);
        }
        #[cfg(desktop)]
        {
            Ok(())
        }
    }

    pub fn scan_pairing_qr(&self) -> Result<String> {
        #[cfg(mobile)]
        {
            let result: ScannedQr = self.handle.run_mobile_plugin("scanPairingQr", ())?;
            return Ok(result.data);
        }
        #[cfg(desktop)]
        {
            Err(Error::Unsupported)
        }
    }

    pub fn pick_folder(&self) -> Result<String> {
        #[cfg(mobile)]
        {
            let result: SelectedUri = self.handle.run_mobile_plugin("pickFolder", ())?;
            return Ok(result.uri);
        }
        #[cfg(desktop)]
        {
            Err(Error::Unsupported)
        }
    }

    pub fn device_name(&self) -> Result<String> {
        #[cfg(mobile)]
        {
            let result: DeviceName = self.handle.run_mobile_plugin("deviceName", ())?;
            return Ok(result.name);
        }
        #[cfg(desktop)]
        {
            Err(Error::Unsupported)
        }
    }
}

pub trait ContentAccessExt<R: Runtime> {
    fn content_access(&self) -> &ContentAccess<R>;
}

impl<R: Runtime, T: Manager<R>> ContentAccessExt<R> for T {
    fn content_access(&self) -> &ContentAccess<R> {
        self.state::<ContentAccess<R>>().inner()
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("content-access")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            let access = ContentAccess {
                handle: api.register_android_plugin(
                    "com.crossdrop.contentaccess",
                    "ContentAccessPlugin",
                )?,
            };
            #[cfg(not(target_os = "android"))]
            let access = {
                let _ = api;
                ContentAccess { _app: app.clone() }
            };
            app.manage(access);
            Ok(())
        })
        .build()
}
