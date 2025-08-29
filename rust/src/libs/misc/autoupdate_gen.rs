use serde::Deserialize;
use std::{fs, path::Path, time::Duration};
use tokio::time::sleep;

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

pub struct AutoUpdateGen {
    version: String,
}

impl AutoUpdateGen {
    pub fn new(version: &str) -> Self {
        AutoUpdateGen {
            version: version.to_string(),
        }
    }

    pub async fn start(mut self) {
        loop {
            self.tick().await;
            sleep(Duration::from_secs(60)).await; // every 60s
        }
    }

    async fn tick(&mut self) {
        if let Err(e) = self.upgrade(false).await {
            eprintln!("upgrade check failed: {}", e);
        }
    }

    async fn upgrade(&mut self, is_boot: bool) -> Result<(), Box<dyn std::error::Error>> {
        let url = "https://api.github.com/repos/amadeus-robot/node/releases/latest";

        let client = reqwest::Client::new();
        let body = client
            .get(url)
            .header("User-Agent", "rust-auto-updater")
            .send()
            .await?
            .text()
            .await?;

        let json: GithubRelease = serde_json::from_str(&body)?;
        let latest_version = json.tag_name.trim_start_matches('v').to_string();

        if self.version < latest_version {
            if let Some(asset) = json
                .assets
                .into_iter()
                .find(|a| a.name == "amadeusd")
            {
                println!("Downloading upgrade from {}", asset.browser_download_url);

                let bin = client
                    .get(&asset.browser_download_url)
                    .send()
                    .await?
                    .bytes()
                    .await?;

                let cwd = std::env::current_dir()?;
                let path_tmp = cwd.join("amadeusd_tmp");
                let path = cwd.join("amadeusd");

                fs::write(&path_tmp, &bin)?;
                fs::rename(&path_tmp, &path)?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mut perms = fs::metadata(&path)?.permissions();
                    perms.set_mode(0o755);
                    fs::set_permissions(&path, perms)?;
                }

                if is_boot {
                    println!("Restarting system after boot upgrade...");
                    std::process::exit(0);
                } else {
                    println!("Upgrade complete, shutting down for restart...");
                    std::process::exit(0);
                }
            }
        }

        Ok(())
    }
}
