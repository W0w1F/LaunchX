use crate::model::{Resource, ResourceType};

pub struct LaunchResult {
    pub name: String,
    pub error: Option<String>,
}

fn open_resource(res: &Resource) -> Result<(), String> {
    let target = res.target.trim();
    match res.kind {
        ResourceType::Url => {
            if !(target.starts_with("http://") || target.starts_with("https://")) {
                return Err(format!("invalid URL: {target}"));
            }
            open::that_detached(target).map_err(|e| e.to_string())
        }
        ResourceType::File => {
            if !std::path::Path::new(target).exists() {
                return Err(format!("file not found: {target}"));
            }
            open::that_detached(target).map_err(|e| e.to_string())
        }
        ResourceType::Folder => {
            if !std::path::Path::new(target).is_dir() {
                return Err(format!("folder not found: {target}"));
            }
            match res.open_with.as_deref() {
                // Open in a specific app, e.g. `code <dir>` for VS Code.
                Some(app) if !app.is_empty() => open_with(target, app),
                // Default: system file explorer.
                _ => open::that_detached(target).map_err(|e| e.to_string()),
            }
        }
    }
}

#[cfg(windows)]
fn open_with(target: &str, app: &str) -> Result<(), String> {
    // Apps like VS Code are launched through `code.cmd` on Windows, which
    // would flash a console window. CREATE_NO_WINDOW suppresses it.
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    std::process::Command::new("cmd")
        .args(["/C", app, target])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(not(windows))]
fn open_with(target: &str, app: &str) -> Result<(), String> {
    open::with_detached(target, app).map_err(|e| e.to_string())
}

/// Launch every resource; one failure never blocks the others.
pub fn launch_all(resources: &[Resource]) -> Vec<LaunchResult> {
    resources
        .iter()
        .map(|res| LaunchResult {
            name: if res.name.is_empty() {
                res.target.clone()
            } else {
                res.name.clone()
            },
            error: open_resource(res).err(),
        })
        .collect()
}
