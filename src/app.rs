use crate::tweaks::Tweak;
use crate::utils;
use crate::utils::execute_command;
use crate::config::Config;
use anyhow::Result;
use ratatui::backend::Backend;
use ratatui::Terminal;
use serde::{Deserialize, Serialize};
use ratatui::widgets::ListState;

pub fn get_app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopLevelCategory {
    pub name: String,
    pub description: String,
    pub tweaks: Vec<Tweak>,
}

impl TopLevelCategory {
    fn new(name: &str, description: &str, tweaks: Vec<Tweak>) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            tweaks,
        }
    }
}

#[derive(Debug, Clone)]
pub struct App {
    pub view_level: u8, // 0: Top-level Categories, 1: Sub-categories/Tweaks
    pub selected_indices: [usize; 2], // [top_level_index, tweak_index]
    pub viewing_sub_category: Option<String>,
    pub should_quit: bool,
    pub categories: Vec<TopLevelCategory>,
    pub applied_tweaks: Vec<String>,
    pub status_message: Option<String>,
    pub status_timer: u32,
    pub pending_destructive_command: Option<(String, String)>, // (tweak_name, command)
    pub confirmation_message: Option<String>,
    pub input_buffer: String,
    pub fullscreen_output: Option<String>,
    pub config: Config,
    pub fullscreen_list: Option<Vec<String>>,
    pub fullscreen_list_state: ListState,
    pub fullscreen_list_title: String,
}

impl App {
    pub fn new() -> App {
        let config = Config::load();
        
        let dock_tweaks = vec![
            Tweak::new("Dock Size", "Change the size of Dock icons", "", "", false),
            Tweak::new("  Small (32px)", "Set Dock icon size to small", "defaults write com.apple.dock tilesize -int 32 && killall Dock", "", false),
            Tweak::new("  Medium (48px)", "Set Dock icon size to medium", "defaults write com.apple.dock tilesize -int 48 && killall Dock", "", false),
            Tweak::new("  Large (64px)", "Set Dock icon size to large", "defaults write com.apple.dock tilesize -int 64 && killall Dock", "", false),
            Tweak::new("Dock Behavior", "Configure Dock behavior settings", "", "", false),
            Tweak::new("  Disable Magnification", "Disable dock magnification effect", "defaults write com.apple.dock magnification -bool false && killall Dock", "", false),
            Tweak::new("  Auto-hide Dock", "Auto-hide the dock", "defaults write com.apple.dock autohide -bool true && killall Dock", "", false),
            Tweak::new("Dock Spacers", "Manage Dock spacers and organization", "", "", false),
            Tweak::new("  Add Small Spacer", "Add a small spacer tile to the Dock", r#"defaults write com.apple.dock persistent-apps -array-add '{"tile-type"="small-spacer-tile";}' && killall Dock"#, "", false),
            Tweak::new("  Remove All Spacers", "Remove all small spacers from the Dock", "defaults write com.apple.dock persistent-apps -array '()' && killall Dock", "", false),
            Tweak::new("Reset Options", "Reset Dock to default settings", "", "", false),
            Tweak::new("  Reset Dock to Default", "Reset Dock to its default settings", "defaults delete com.apple.dock && killall Dock", "", false),
        ];
        
        let animated_wallpapers_tweaks = vec![
            Tweak::new("Video Wallpaper (mpv)", "Set a video as your wallpaper (requires mpv)", "", "", false),
            Tweak::new("  Play video as wallpaper (experimental)", "Play ~/Movies/wallpaper.mp4 as wallpaper (requires mpv)", "mpv --wid=$(osascript -e 'tell application \"Finder\" to get id of window 1') --loop --no-border --geometry=100%:100% --panscan=1.0 --no-osc --no-input-default-bindings --no-audio ~/Movies/wallpaper.mp4", "", false),
        ];

        let power_management_tweaks = vec![
            Tweak::new("Computer Sleep", "Adjust computer sleep settings", "", "", false),
            Tweak::new("  Never", "Prevent computer from sleeping", "sudo systemsetup -setcomputersleep Never", "", false),
            Tweak::new("  15 minutes (Default)", "Set computer sleep timer to 15 minutes", "sudo systemsetup -setcomputersleep 15", "", false),
            Tweak::new("  30 minutes", "Set computer sleep timer to 30 minutes", "sudo systemsetup -setcomputersleep 30", "", false),
            Tweak::new("  1 hour", "Set computer sleep timer to 60 minutes", "sudo systemsetup -setcomputersleep 60", "", false),
            Tweak::new("Display Sleep", "Adjust display sleep settings", "", "", false),
            Tweak::new("  5 minutes", "Set display sleep timer to 5 minutes", "sudo systemsetup -setdisplaysleep 5", "", false),
            Tweak::new("  10 minutes (Default)", "Set display sleep timer to 10 minutes", "sudo systemsetup -setdisplaysleep 10", "", false),
            Tweak::new("  15 minutes", "Set display sleep timer to 15 minutes", "sudo systemsetup -setdisplaysleep 15", "", false),
            Tweak::new("  Never", "Prevent display from sleeping", "sudo systemsetup -setdisplaysleep Never", "", false),
        ];

        let network_tweaks = vec![
            Tweak::new("Flush DNS Cache", "Removes all entries from the DNS cache", "sudo dscacheutil -flushcache; sudo killall -HUP mDNSResponder", "", false),
        ];

        let optimization_tweaks = vec![
            Tweak::new("Clean Up Caches", "Remove temporary cache files", "", "", false),
            Tweak::new("  Clear User Cache (destructive)", "Removes all files from ~/Library/Caches", "rm -rf ~/Library/Caches/*", "", false),
            Tweak::new("  Clear System Cache (destructive)", "Removes all files from /Library/Caches", "sudo rm -rf /Library/Caches/*", "", false),

            Tweak::new("Organize Desktop", "Move files from Desktop to organized folders", "", "", false),
            Tweak::new("  Move screenshots to Pictures folder", "Finds all screenshots on Desktop and moves them to ~/Pictures/Screenshots", "mkdir -p ~/Pictures/Screenshots && find ~/Desktop -maxdepth 1 \\( -name 'Screen Shot*.png' -o -name 'Screenshot*.png' \\) -exec mv -n {} ~/Pictures/Screenshots/ \\;", "", false),
            Tweak::new("  Move project folders to ~/Developer", "Moves folders with .git, .gitignore, or source code", "zsh scripts/organize_projects.sh", "", false),
            Tweak::new("  Move images to ~/Pictures", "Moves common image files from Desktop to Pictures", "find ~/Desktop -maxdepth 1 -type f \\( -iname '*.png' -o -iname '*.jpg' -o -iname '*.jpeg' -o -iname '*.gif' \\) -exec mv -n {} ~/Pictures/ \\;", "", false),
            Tweak::new("  Move videos to ~/Movies", "Moves common video files from Desktop to Movies", "find ~/Desktop -maxdepth 1 -type f \\( -iname '*.mov' -o -iname '*.mp4' \\) -exec mv -n {} ~/Movies/ \\;", "", false),
            Tweak::new("  Move documents to ~/Documents", "Moves common document files from Desktop to Documents", "find ~/Desktop -maxdepth 1 -type f \\( -iname '*.pdf' -o -iname '*.docx' \\) -exec mv -n {} ~/Documents/ \\;", "", false),

            Tweak::new("Find Large Files", "Identify large files to free up space", "", "", false),
            Tweak::new("  List 10 largest files in Home", "Shows a list of the 10 biggest files in your home directory.", "echo 'Large files in home directory:' && ls -lah ~ | grep -v '^d' | sort -k5 -hr | head -n 10", "", false),
        ];

        let brew_tweaks = vec![
            Tweak::new("Brew Installation", "Manage Homebrew installation", "", "", false),
            Tweak::new("  Install Homebrew (interactive)", "Install Homebrew package manager", "curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh | bash", "", false),
            Tweak::new("  Uninstall Homebrew (destructive)", "Remove Homebrew and all packages (destructive)", "curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/uninstall.sh | bash", "", false),
            Tweak::new("  Check Homebrew Status", "Check if Homebrew is installed and working", "__CHECK_BREW__", "", false),
            
            Tweak::new("Brew Maintenance", "Maintain and update Homebrew", "", "", false),
            Tweak::new("  Update Homebrew", "Update Homebrew and all packages", "brew update && brew upgrade", "", false),
            Tweak::new("  Clean Up Homebrew", "Remove old versions and clean cache", "brew cleanup", "", false),
            Tweak::new("  List Installed Packages", "View all installed Homebrew packages", "__LIST_INSTALLED__", "", false),
            Tweak::new("  List Outdated Packages", "View packages that have updates available", "__LIST_OUTDATED__", "", false),
            
            Tweak::new("Brew Analytics", "Manage Homebrew analytics", "", "", false),
            Tweak::new("  Disable Analytics", "Disable Homebrew analytics collection", "brew analytics off", "", false),
            Tweak::new("  Enable Analytics", "Enable Homebrew analytics collection", "brew analytics on", "", false),
            Tweak::new("  Show Analytics Status", "Check if analytics are enabled", "brew analytics state", "", false),
        ];

        let about_tweaks = vec![
            Tweak::new("Application Info", "Information about this application", "", "", false),
            Tweak::new("  Version", "Show application version", "__SHOW_VERSION__", "", false),
            Tweak::new("  About", "Show detailed information about the application", "echo 'macOS Tweaks - A terminal-based GUI for managing macOS system tweaks and optimizations.\\n\\nBuilt with Rust and Ratatui.\\n\\nFeatures:\\n- Tabbed interface with organized categories\\n- Interactive navigation\\n- Real-time status updates\\n- Customizable color schemes\\n- Safe system modifications\\n\\nAuthor: Doruk Sarp Aydın\\nLicense: MIT'", "", false),
            Tweak::new("  System Information", "Show system information", "sw_vers && echo '\\n---\\n' && system_profiler SPHardwareDataType | grep -E '(Model Name|Model Identifier|Processor|Memory|Serial Number)'", "", false),
            Tweak::new("  Dependencies", "Show application dependencies", "echo 'Dependencies:\\n- Rust (latest stable)\\n- ratatui (terminal UI framework)\\n- crossterm (terminal manipulation)\\n- serde (serialization)\\n- anyhow (error handling)'", "", false),
        ];

        let categories = vec![
            TopLevelCategory::new("Dock", "Customize macOS Dock settings", dock_tweaks),
            TopLevelCategory::new("Animated Wallpapers", "Enable animated wallpapers", animated_wallpapers_tweaks),
            TopLevelCategory::new("Power Management", "Configure sleep and power settings", power_management_tweaks),
            TopLevelCategory::new("Networking", "Configure network settings", network_tweaks),
            TopLevelCategory::new("Optimization", "Apply system performance tweaks", optimization_tweaks),
            TopLevelCategory::new("Brew Management", "Manage Homebrew package manager", brew_tweaks),
            TopLevelCategory::new("About", "Application information and system details", about_tweaks),
        ];

        App {
            view_level: 0,
            selected_indices: [0, 0],
            viewing_sub_category: None,
            should_quit: false,
            categories,
            applied_tweaks: Vec::new(),
            status_message: None,
            status_timer: 0,
            pending_destructive_command: None,
            confirmation_message: None,
            input_buffer: String::new(),
            fullscreen_output: None,
            config,
            fullscreen_list: None,
            fullscreen_list_state: ListState::default(),
            fullscreen_list_title: String::new(),
        }
    }

    /// Returns the list of items to be displayed based on the current view level.
    pub fn get_current_list_items(&self) -> Vec<String> {
        match self.view_level {
            0 => self.categories.iter().map(|c| c.name.clone()).collect(),
            1 => {
                let current_cat_tweaks = &self.categories[self.selected_indices[0]].tweaks;
                if let Some(sub_cat_name) = &self.viewing_sub_category {
                    // Viewing options within a sub-category
                    current_cat_tweaks.iter()
                        .skip_while(|t| &t.name != sub_cat_name)
                        .skip(1)
                        .take_while(|t| t.name.starts_with("  "))
                        .map(|t| t.name.clone())
                        .collect()
                } else {
                    // Viewing sub-categories
                    current_cat_tweaks.iter()
                        .filter(|t| !t.name.starts_with("  "))
                        .map(|t| t.name.clone())
                        .collect()
                }
            },
            _ => vec![],
        }
    }

    /// Gets the currently selected tweak or sub-category.
    pub fn get_selected_item(&self) -> Option<Tweak> {
        match self.view_level {
            1 => {
                let list = self.get_current_list_items();
                let selected_name = list.get(self.selected_indices[1])?;
                let current_cat_tweaks = &self.categories[self.selected_indices[0]].tweaks;
                current_cat_tweaks.iter().find(|t| &t.name == selected_name).cloned()
            },
            _ => None,
        }
    }

    pub fn next_item(&mut self) {
        let count = self.get_current_list_items().len();
        let index = if self.view_level == 0 { &mut self.selected_indices[0] } else { &mut self.selected_indices[1] };
        if count > 0 {
            *index = (*index + 1) % count;
        }
    }
    
    pub fn previous_item(&mut self) {
        let count = self.get_current_list_items().len();
        let index = if self.view_level == 0 { &mut self.selected_indices[0] } else { &mut self.selected_indices[1] };
        if count > 0 {
            *index = if *index == 0 { count - 1 } else { *index - 1 };
        }
    }
    
    pub fn handle_right_key(&mut self) {
        match self.view_level {
            0 => { // From top-level to sub-categories
                if !self.categories[self.selected_indices[0]].tweaks.is_empty() {
                    self.view_level = 1;
                    self.selected_indices[1] = 0;
                } else {
                    self.status_message = Some("This category is empty.".to_string());
                    self.status_timer = 50;
                }
            },
            1 => { // From sub-categories to options
                if self.viewing_sub_category.is_none() {
                    if let Some(item) = self.get_selected_item() {
                        if item.enable_command.is_empty() {
                            self.viewing_sub_category = Some(item.name.clone());
                            self.selected_indices[1] = 0;
                        }
                    }
                }
            },
            _ => {}
        }
    }
    
    pub fn handle_left_key(&mut self) {
        match self.view_level {
            1 => {
                if self.viewing_sub_category.is_some() {
                    self.viewing_sub_category = None;
                    self.selected_indices[1] = 0;
                } else {
                    self.view_level = 0;
                }
            },
            _ => {}
        }
    }
    
    pub fn apply_selected_tweak<B: Backend + std::io::Write>(
        &mut self,
        terminal: &mut Terminal<B>,
        run_interactive: impl Fn(&mut Terminal<B>, &str) -> Result<()>,
    ) -> Result<()> {
        if self.view_level == 1 {
            if let Some(tweak) = self.get_selected_item() {
                if tweak.enable_command == "__SHOW_VERSION__" {
                    self.fullscreen_output = Some(format!("macOS Tweaks v{}", get_app_version()));
                    return Ok(());
                }
                if tweak.enable_command == "__CHECK_BREW__" {
                    let message = if utils::check_command_exists("brew") {
                        "Homebrew is installed and available in your PATH."
                    } else {
                        "Homebrew is not installed or not in your PATH."
                    };
                    self.fullscreen_output = Some(message.to_string());
                    return Ok(());
                }
                if tweak.enable_command == "__LIST_INSTALLED__" {
                    match utils::execute_command("brew list", false) {
                        Ok(output) => {
                            let packages: Vec<String> = output.lines().filter(|l| !l.trim().is_empty()).map(String::from).collect();
                            if packages.is_empty() {
                                self.fullscreen_output = Some("No installed Homebrew packages found.".to_string());
                            } else {
                                self.fullscreen_list = Some(packages);
                                self.fullscreen_list_state.select(Some(0));
                                self.fullscreen_list_title = "Installed Packages (Press Enter for info)".to_string();
                            }
                        }
                        Err(e) => {
                            self.fullscreen_output = Some(format!("Error fetching installed packages: {}", e));
                        }
                    }
                    return Ok(());
                }
                if tweak.enable_command == "__LIST_OUTDATED__" {
                    match utils::execute_command("brew outdated", false) {
                        Ok(output) => {
                            let packages: Vec<String> = output.lines().filter(|l| !l.trim().is_empty()).map(String::from).collect();
                            if packages.is_empty() {
                                self.fullscreen_output = Some("All Homebrew packages are up to date.".to_string());
                            } else {
                                self.fullscreen_list = Some(packages);
                                self.fullscreen_list_state.select(Some(0));
                                self.fullscreen_list_title = "Outdated Packages (Press Enter to upgrade)".to_string();
                            }
                        }
                        Err(e) => {
                            self.fullscreen_output = Some(format!("Error fetching outdated packages: {}", e));
                        }
                    }
                    return Ok(());
                }
                if tweak.enable_command.is_empty() {
                    self.handle_right_key();
                    return Ok(());
                }

                let tweak_name = tweak.name.clone();
                let command = tweak.enable_command.clone();
                let can_run_multiple = tweak_name.contains("Add Small Spacer");
                let is_info_command = tweak_name.contains("List") || tweak_name.contains("Show") || tweak_name.contains("About") || tweak_name.contains("Version") || tweak_name.contains("Dependencies") || tweak_name.contains("System Information");
                let is_destructive = tweak_name.contains("(destructive)");
                let is_interactive = utils::require_sudo(&command) || is_destructive || tweak_name.contains("(interactive)");

                if is_destructive {
                    self.pending_destructive_command = Some((tweak_name.clone(), command.clone()));
                    self.confirmation_message = Some(format!("⚠️  DESTRUCTIVE ACTION: {}\nType 'yes' to confirm or press any other key to cancel", tweak_name.trim()));
                    return Ok(());
                }

                // Debug: Show what type of command this is
                let command_type = if is_info_command { "info" } else if is_interactive { "interactive" } else { "normal" };
                self.status_message = Some(format!("Executing {} command: {}", command_type, tweak_name.trim()));
                self.status_timer = 20;

                if is_interactive {
                    run_interactive(terminal, &command)?;
                    self.status_message = Some(format!("Successfully applied: {}", tweak_name.trim()));
                    self.status_timer = 50;
                } else {
                    match execute_command(&command, false) {
                        Ok(output) => {
                            if is_info_command {
                                let final_output = if output.trim().is_empty() {
                                    format!("'{}' executed successfully with no output.", tweak_name.trim())
                                } else {
                                    output
                                };
                                self.fullscreen_output = Some(final_output);
                            } else {
                                if !can_run_multiple && !self.applied_tweaks.contains(&tweak_name) {
                                    self.applied_tweaks.push(tweak_name.clone());
                                }
                                self.status_message = Some(format!("Successfully applied: {}", tweak_name.trim()));
                                self.status_timer = 50;
                            }
                        }
                        Err(e) => {
                            self.status_message = Some(format!("Error executing '{}': {}", tweak_name.trim(), e));
                            self.status_timer = 80;
                        }
                    }
                }
            }
        } else {
             // If user presses enter on a category, treat it like right arrow
            self.handle_right_key();
        }
        Ok(())
    }

    pub fn handle_confirmation<B: Backend>(
        &mut self,
        input: &str,
        terminal: &mut Terminal<B>,
        run_interactive: impl Fn(&mut Terminal<B>, &str) -> Result<()>,
    ) -> Result<()> {
        if let Some((tweak_name, command)) = self.pending_destructive_command.clone() {
            if input.trim().to_lowercase() == "yes" {
                // User confirmed, execute the destructive command
                run_interactive(terminal, &command)?;
                self.status_message = Some(format!("Successfully applied: {}", tweak_name.trim()));
                self.status_timer = 50;
            } else {
                self.status_message = Some("Action canceled.".to_string());
                self.status_timer = 50;
            }
        }
        self.pending_destructive_command = None;
        self.confirmation_message = None;
        Ok(())
    }

    pub fn update_status_timer(&mut self) {
        if self.status_timer > 0 {
            self.status_timer -= 1;
            if self.status_timer == 0 {
                self.status_message = None;
            }
        }
    }

    pub fn find_tweak_by_name(&self, name: &str) -> Option<Tweak> {
        self.categories
            .iter()
            .flat_map(|category| &category.tweaks)
            .find(|tweak| tweak.name.trim().eq_ignore_ascii_case(name.trim()))
            .cloned()
    }
} 