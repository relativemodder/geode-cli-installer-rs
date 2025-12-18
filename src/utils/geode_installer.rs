use crate::utils::steam_game_finder::SteamGameFinder;
use reqwest::blocking::Client;
use serde_json::Value;
use std::fs::{self, File};
use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use zip::ZipArchive;
use std::os::unix::fs::PermissionsExt;


pub struct GeodeInstaller {
    finder: SteamGameFinder,
    client: Client,
}

impl GeodeInstaller {
    pub fn new() -> Result<Self, String> {
        let client = Client::builder()
            .build()
            .map_err(|err| format!("Failed to create HTTP client: {}", err))?;

        Ok(
            GeodeInstaller { finder: SteamGameFinder::new(), client }
        )
    }

    fn make_http_request(&self, url: &str) -> Result<String, String> {
        let response = self.client.get(url).send()
            .map_err(|err| format!("request failed: {}", err))?;

        if !response.status().is_success() {
            return Err(format!("request returned HTTP error code: {}", response.status().as_str()));
        }

        response.text().map_err(|err|format!("can't read response (wtf): {}", err))
    }

    fn download_file(&self, url: &str, output_path: &Path) -> Result<(), String> {
        let mut response = self.client.get(url).send()
            .map_err(|err| format!("download failed: {}", err))?;

        if !response.status().is_success() {
            return Err(format!("request returned HTTP error code: {}", response.status().as_str()));
        }

        let mut file = File::create(output_path)
            .map_err(|err| format!("uh-oh, failed to create file: {}", err))?;

        response.copy_to(&mut file)
            .map_err(|err| format!("uh-oh, failed to write file: {}", err))?;

        Ok(())
    }

    fn extract_zip(&self, zip_path: &Path, destination: &Path) -> Result<(), String> {
        let file = File::open(zip_path)
            .map_err(|e| format!("failed to open zip file: {}", e))?;

        let mut archive = ZipArchive::new(file)
            .map_err(|e| format!("failed to read zip arc: {}", e))?;

        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| format!("failed to access zip entry: {}", e))?;

            let out_path = match file.enclosed_name() {
                Some(path) => destination.join(path),
                None => continue
            };

            if file.name().ends_with('/') {
                fs::create_dir_all(&out_path)
                    .map_err(|e| format!("failed to create directory: {}", e))?;
            } else {
                if let Some(parent) = out_path.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("Failed to create parent directory: {}", e))?;
                }

                let mut out_file = File::create(&out_path)
                    .map_err(|e| format!("Failed to create output file: {}", e))?;

                io::copy(&mut file, &mut out_file)
                    .map_err(|e| format!("Failed to extract file: {}", e))?;
            }

            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&out_path, fs::Permissions::from_mode(mode))
                    .ok();
            }

        }

        Ok(())
    }

    pub fn get_latest_geode_tag(&self) -> Result<String, String> {
        let url = "https://api.geode-sdk.org/v1/loader/versions/latest";
        let response = self.make_http_request(url)?;

        let json: Value = serde_json::from_str(&response)
            .map_err(|op| format!("failed to parse json: {}", op))?;

        if let Some(error) = json["error"].as_str() {
            if !error.is_empty() {
                return Err(format!("Geode's API is not in mood today: {}", error));
            }
        }

        let tag = json["payload"]["tag"]
            .as_str()
            .ok_or_else(|| "failed to get tag from response".to_string())?;

        Ok(tag.to_string())
    }

    pub fn get_download_url(&self) -> Result<String, String> {
        let tag = self.get_latest_geode_tag()?;
        Ok(format!(
            "https://github.com/geode-sdk/geode/releases/download/{}/geode-{}-win.zip",
            tag, tag
        ))
    }

    pub fn unzip_to_destination(
        &self,
        zip_url: &str,
        destination_dir: &Path
    ) -> Result<(), String> {
        let zip_file_path = destination_dir.join("geode_win.zip");

        fs::create_dir_all(destination_dir)
            .map_err(|err| format!("failed to create destination dir: {}", err))?;

        println!("Downloading geode_win.zip from {}...", zip_url);

        self.download_file(zip_url, &zip_file_path)?;
        self.extract_zip(&zip_file_path, destination_dir)?;

        fs::remove_file(zip_file_path)
            .map_err(|err| format!("failed to remove zip file: {}", err))?;

        Ok(())
    }

    pub fn install_to_dir(&self, destination_dir: &Path) -> Result<(), String> {
        let win_zip_release_link = self.get_download_url()?;

        println!("Get ready to download Geode...");

        self.unzip_to_destination(&win_zip_release_link, destination_dir)?;

        Ok(())
    }

    fn get_current_timestamp(&self) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now.to_string()
    }

    fn get_hex_timestamp(&self) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        format!("{:x}", now)
    }

    pub fn patch_prefix_registry(&self, reg_file_path: &Path) -> Result<(), String> {
        if !reg_file_path.exists() {
            return Err(format!("Registry file not found: {:?}", reg_file_path));
        }

        let mut content = fs::read_to_string(reg_file_path)
            .map_err(|e| format!("Failed to read registry file: {}", e))?;

        let dll_overrides_section = "[Software\\\\Wine\\\\DllOverrides]";
        let xinput_entry = "\"xinput1_4\"=\"native,builtin\"";

        if !content.contains(dll_overrides_section) {
            // Section doesn't exist, add it
            content.push_str(&format!(
                "\n\n{} {}\n",
                dll_overrides_section,
                self.get_current_timestamp()
            ));
            content.push_str(&format!("#time={}\n", self.get_hex_timestamp()));
            content.push_str(&format!("{}\n", xinput_entry));
        } else if !content.contains("\"xinput1_4\"=") {
            // Section exists but entry doesn't
            if let Some(section_pos) = content.find(dll_overrides_section) {
                let search_start = section_pos + dll_overrides_section.len();
                if let Some(next_section_pos) = content[search_start..].find("\n[") {
                    let insert_pos = search_start + next_section_pos;
                    content.insert_str(insert_pos, &format!("{}\n", xinput_entry));
                } else {
                    content.push_str(&format!("\n{}\n", xinput_entry));
                }
            }
        }

        fs::write(reg_file_path, content)
            .map_err(|e| format!("Failed to write registry file: {}", e))?;

        Ok(())
    }

    pub fn install_geode_to_wine(&self, prefix: &Path, gd_path: &Path) -> Result<(), String> {
        if !prefix.exists() {
            return Err(format!("Can't find prefix: {:?}", prefix));
        }

        if !gd_path.exists() {
            return Err(format!("Can't find Geometry Dash: {:?}", gd_path));
        }

        println!("Installing Geode to: {:?}", gd_path);
        self.install_to_dir(gd_path)?;

        println!("Patching Wine registry...");
        let user_reg = prefix.join("user.reg");
        self.patch_prefix_registry(&user_reg)?;

        println!("Geode installation completed!");

        Ok(())
    }

    pub fn install_geode_to_steam(&self) -> Result<(), String> {
        if self.finder.get_steam_root().is_none() {
            return Err("Can't find Steam Root".to_string());
        }

        println!(
            "Steam root found at: {:?}",
            self.finder.get_steam_root().as_ref().unwrap()
        );

        // GD appid is 322170
        let gd_info = self.finder.get_game_info("322170");

        if !gd_info.found {
            return Err("Can't find Geometry Dash.".to_string());
        }

        let game_path = gd_info
            .game_path
            .as_ref()
            .ok_or_else(|| "Game path is missing".to_string())?;

        println!("Geometry Dash found at: {:?}", game_path);

        let proton_prefix = gd_info
            .proton_prefix
            .as_ref()
            .ok_or_else(|| "Can't find Proton Prefix.".to_string())?;

        println!("Proton prefix found at: {:?}", proton_prefix);

        if !game_path.exists() {
            return Err(format!("Can't find Steam GD at {:?}", game_path));
        }

        self.install_geode_to_wine(proton_prefix, game_path)?;

        Ok(())
    }
}

impl Default for GeodeInstaller {
    fn default() -> Self {
        Self::new().expect("failed to create GeodeInstaller")
    }
}