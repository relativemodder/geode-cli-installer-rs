use homedir::my_home;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct GameInfo {
    #[allow(unused)]
    pub app_id: String,
    pub game_path: Option<PathBuf>,
    pub proton_prefix: Option<PathBuf>,
    pub library_path: Option<PathBuf>,
    pub found: bool,
}

pub struct SteamGameFinder {
    steam_root: Option<PathBuf>,
    library_folders: Vec<PathBuf>,
}

impl SteamGameFinder {
    pub fn new() -> Self {
        let steam_root = Self::find_steam_root().ok();
        let library_folders = Self::initialize_library_folders(&steam_root);
        
        SteamGameFinder {
            steam_root,
            library_folders,
        }
    }

    pub fn get_steam_root(&self) -> &Option<PathBuf> {
        &self.steam_root
    }

    #[allow(unused)]
    pub fn get_library_folders(&self) -> &Vec<PathBuf> {
        &self.library_folders
    }

    fn find_steam_root() -> Result<PathBuf, String> {
        let home_dir = my_home()
            .map_err(|e| e.to_string())?
            .ok_or_else(|| String::from("Home dir is empty somehow"))?;

        let possible_paths = vec![
            home_dir.join(".steam").join("steam"),
            home_dir.join(".steam").join("root"),
            home_dir.join(".local").join("share").join("Steam"),
            home_dir.join(".var").join("app").join("com.valvesoftware.Steam"), // let's be honest. Steam Flatpak users, do you hate urself?
            home_dir.join(".var").join("app").join("com.valvesoftware.Steam").join("data").join("Steam"),
            PathBuf::from("/usr/share/steam"),
        ];

        for path in possible_paths {
            if path.exists() && path.join("steamapps").exists() {
                return Ok(path);
            }
        }

        Err(String::from("Can't find Steam root"))
    }

    fn parse_vdf_file(file_path: &PathBuf) -> HashMap<String, String> {
        let mut result = HashMap::new();

        if !file_path.exists() {
            return result;
        }

        let content = match fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(_) => return result,
        };

        let mut pos = 0;
        Self::parse_vdf_recursive(&content, &mut pos, &mut result, String::new());

        result
    }

    fn parse_vdf_recursive(
        content: &str,
        pos: &mut usize,
        result: &mut HashMap<String, String>,
        prefix: String,
    ) {
        let chars: Vec<char> = content.chars().collect();

        while *pos < chars.len() {
            // Skip whitespace
            while *pos < chars.len() && chars[*pos].is_whitespace() {
                *pos += 1;
            }

            if *pos >= chars.len() {
                break;
            }

            // Handle comments
            if *pos + 1 < chars.len() && chars[*pos] == '/' && chars[*pos + 1] == '/' {
                while *pos < chars.len() && chars[*pos] != '\n' {
                    *pos += 1;
                }
                continue;
            }

            // Handle closing brace
            if chars[*pos] == '}' {
                *pos += 1;
                return;
            }

            // Handle opening brace
            if chars[*pos] == '{' {
                *pos += 1;
                continue;
            }

            // Parse key-value pairs
            if chars[*pos] == '"' {
                *pos += 1;

                // Read key
                let mut key = String::new();
                while *pos < chars.len() && chars[*pos] != '"' {
                    key.push(chars[*pos]);
                    *pos += 1;
                }
                *pos += 1;

                // Skip whitespace
                while *pos < chars.len() && chars[*pos].is_whitespace() {
                    *pos += 1;
                }

                if *pos < chars.len() && chars[*pos] == '"' {
                    // Read value
                    *pos += 1;
                    let mut value = String::new();
                    while *pos < chars.len() && chars[*pos] != '"' {
                        value.push(chars[*pos]);
                        *pos += 1;
                    }
                    *pos += 1;

                    let full_key = if prefix.is_empty() {
                        key
                    } else {
                        format!("{}.{}", prefix, key)
                    };
                    result.insert(full_key, value);
                } else if *pos < chars.len() && chars[*pos] == '{' {
                    // Nested object
                    *pos += 1;
                    let new_prefix = if prefix.is_empty() {
                        key
                    } else {
                        format!("{}.{}", prefix, key)
                    };
                    Self::parse_vdf_recursive(content, pos, result, new_prefix);
                }
            } else {
                *pos += 1;
            }
        }
    }

    fn initialize_library_folders(steam_root: &Option<PathBuf>) -> Vec<PathBuf> {
        let steam_root = match steam_root {
            Some(root) => root,
            None => return Vec::new(),
        };

        let mut folders = vec![steam_root.join("steamapps")];

        let library_file = steam_root.join("steamapps").join("libraryfolders.vdf");
        if library_file.exists() {
            let data = Self::parse_vdf_file(&library_file);

            for (key, value) in data.iter() {
                if (key.starts_with("libraryfolders.") && key.contains(".path"))
                    || key.contains(".path")
                {
                    let path = PathBuf::from(value).join("steamapps");
                    if path.exists() {
                        folders.push(path);
                    }
                }
            }
        }

        // Remove duplicates
        let mut unique_paths = HashSet::new();
        let mut unique_folders = Vec::new();

        for folder in folders {
            let path_str = folder.to_string_lossy().to_string();
            if !unique_paths.contains(&path_str) {
                unique_paths.insert(path_str);
                unique_folders.push(folder);
            }
        }

        unique_folders
    }

    pub fn find_game_by_appid(&self, app_id: &str) -> Option<(PathBuf, PathBuf)> {
        for library_path in &self.library_folders {
            let acf_file = library_path.join(format!("appmanifest_{}.acf", app_id));

            if acf_file.exists() {
                let acf_data = Self::parse_vdf_file(&acf_file);

                if let Some(install_dir) = acf_data.get("AppState.installdir") {
                    let game_path = library_path.join("common").join(install_dir);

                    if game_path.exists() {
                        return Some((game_path, library_path.clone()));
                    }
                }
            }
        }

        None
    }

    pub fn find_proton_prefix(
        &self,
        app_id: &str,
        library_path: Option<&PathBuf>,
    ) -> Option<PathBuf> {
        if let Some(lib_path) = library_path {
            let compatdata_path = lib_path
                .join("compatdata")
                .join(app_id)
                .join("pfx");
            if compatdata_path.exists() {
                return Some(compatdata_path);
            }
        }

        for lib_path in &self.library_folders {
            let compatdata_path = lib_path
                .join("compatdata")
                .join(app_id)
                .join("pfx");
            if compatdata_path.exists() {
                return Some(compatdata_path);
            }
        }

        None
    }

    pub fn get_game_info(&self, app_id: &str) -> GameInfo {
        let mut result = GameInfo {
            app_id: app_id.to_string(),
            game_path: None,
            proton_prefix: None,
            library_path: None,
            found: false,
        };

        if let Some((game_path, library_path)) = self.find_game_by_appid(app_id) {
            result.game_path = Some(game_path);
            result.library_path = Some(library_path.clone());
            result.found = true;

            if let Some(proton_prefix) = self.find_proton_prefix(app_id, Some(&library_path)) {
                result.proton_prefix = Some(proton_prefix);
            }
        }

        result
    }
}
