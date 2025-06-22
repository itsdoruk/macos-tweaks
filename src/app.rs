use crate::tweaks::Tweak;
use crate::utils;
use crate::utils::execute_command;
use crate::config::Config;
use anyhow::Result;
use ratatui::backend::Backend;
use ratatui::Terminal;
use serde::{Deserialize, Serialize};
use ratatui::widgets::ListState;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

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
pub enum Tile {
    Wall,
    Floor,
    Target,
}

#[derive(Debug, Clone)]
pub struct SokobanGame {
    pub level: Vec<Vec<Tile>>,
    pub player: (usize, usize),
    pub boxes: Vec<(usize, usize)>,
    pub moves: u32,
    pub is_complete: bool,
    width: usize,
    height: usize,
}

impl SokobanGame {
    pub fn new() -> Self {
        // A new level with a bit more space
   // The level from the image
   let level_layout = vec![
    "  ########",
    "  #..    #",
    "  #@$    #",
    "  #####  #",
    "    # $  #",
    "    #    #",
    "    ######",
];
        let mut level = Vec::new();
        let mut player = (0, 0);
        let mut boxes = Vec::new();
        let height = level_layout.len();
        let width = level_layout.iter().map(|r| r.len()).max().unwrap_or(0);

        for (y, row_str) in level_layout.iter().enumerate() {
            let mut row = Vec::new();
            for (x, char) in row_str.chars().enumerate() {
                match char {
                    '#' => row.push(Tile::Wall),
                    '@' => {
                        player = (x, y);
                        row.push(Tile::Floor);
                    }
                    '$' => {
                        boxes.push((x, y));
                        row.push(Tile::Floor);
                    }
                    '.' => row.push(Tile::Target),
                    _ => row.push(Tile::Floor),
                }
            }
            // Pad shorter rows
            while row.len() < width {
                row.push(Tile::Floor);
            }
            level.push(row);
        }

        let mut game = SokobanGame {
            level,
            player,
            boxes,
            moves: 0,
            is_complete: false,
            width,
            height,
        };
        game.check_win_condition();
        game
    }

    pub fn move_player(&mut self, dx: i32, dy: i32) {
        if self.is_complete {
            return;
        }

        let new_x = (self.player.0 as i32 + dx) as usize;
        let new_y = (self.player.1 as i32 + dy) as usize;

        if new_x >= self.width || new_y >= self.height || matches!(self.level[new_y][new_x], Tile::Wall) {
            return;
        }

        if let Some(box_index) = self.boxes.iter().position(|&b| b == (new_x, new_y)) {
            let new_box_x = (new_x as i32 + dx) as usize;
            let new_box_y = (new_y as i32 + dy) as usize;

            if new_box_x >= self.width || new_box_y >= self.height || matches!(self.level[new_box_y][new_box_x], Tile::Wall) || self.boxes.contains(&(new_box_x, new_box_y)) {
                return;
            }
            self.boxes[box_index] = (new_box_x, new_box_y);
        }

        self.player = (new_x, new_y);
        self.moves += 1;
        self.check_win_condition();
    }

    fn check_win_condition(&mut self) {
        self.is_complete = self.level.iter().enumerate().all(|(y, row)| {
            row.iter().enumerate().all(|(x, tile)| {
                if let Tile::Target = tile {
                    self.boxes.contains(&(x, y))
                } else {
                    true
                }
            })
        });
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

#[derive(Debug, Clone)]
pub struct App {
    pub view_level: u8, // 0: Top-level Categories, 1: Sub-categories/Tweaks
    pub selected_indices: [usize; 2], // [top_level_index, tweak_index]
    pub category_list_state: ListState,
    pub tweak_list_state: ListState,
    pub viewing_sub_category: Option<String>,
    pub should_quit: bool,
    pub categories: Vec<TopLevelCategory>,
    pub applied_tweaks: Vec<String>,
    pub status_message: Option<String>,
    pub status_timer: u32,
    pub pending_destructive_command: Option<(String, String)>, // (tweak_name, command)
    pub confirmation_message: Option<String>,
    pub text_input_prompt: Option<String>,
    pub text_input_command_template: Option<String>,
    pub input_buffer: String,
    pub fullscreen_output: Option<String>,
    pub fullscreen_output_scroll: u16,
    pub config: Config,
    pub fullscreen_list: Option<Vec<String>>,
    pub fullscreen_list_state: ListState,
    pub fullscreen_list_title: String,
    pub sokoban_game: Option<SokobanGame>,
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
            Tweak::new("  Show Hidden Apps", "Show hidden applications in Dock", "defaults write com.apple.dock showhidden -bool true && killall Dock", "", false),
            Tweak::new("  Disable App Bouncing", "Disable app icon bouncing", "defaults write com.apple.dock no-bouncing -bool true && killall Dock", "", false),
            Tweak::new("Dock Spacers", "Manage Dock spacers and organization", "", "", false),
            Tweak::new("  Add Small Spacer", "Add a small spacer tile to the Dock", r#"defaults write com.apple.dock persistent-apps -array-add '{"tile-type"="small-spacer-tile";}' && killall Dock"#, "", false),
            Tweak::new("  Add Large Spacer", "Add a large spacer tile to the Dock", r#"defaults write com.apple.dock persistent-apps -array-add '{"tile-type"="spacer-tile";}' && killall Dock"#, "", false),
            Tweak::new("  Remove All Spacers", "Remove all spacers from the Dock", "defaults write com.apple.dock persistent-apps -array '()' && killall Dock", "", false),
            Tweak::new("Dock Position", "Change Dock position", "", "", false),
            Tweak::new("  Position Left", "Move Dock to left side", "defaults write com.apple.dock orientation -string left && killall Dock", "", false),
            Tweak::new("  Position Bottom", "Move Dock to bottom (default)", "defaults write com.apple.dock orientation -string bottom && killall Dock", "", false),
            Tweak::new("  Position Right", "Move Dock to right side", "defaults write com.apple.dock orientation -string right && killall Dock", "", false),
            Tweak::new("Reset Options", "Reset Dock to default settings", "", "", false),
            Tweak::new("  Reset Dock to Default", "Reset Dock to its default settings", "defaults delete com.apple.dock && killall Dock", "", false),
        ];
        
        let finder_tweaks = vec![
            Tweak::new("Finder Appearance", "Customize Finder appearance", "", "", false),
            Tweak::new("  Show Hidden Files", "Show hidden files in Finder", "defaults write com.apple.finder AppleShowAllFiles -bool true && killall Finder", "", false),
            Tweak::new("  Hide Hidden Files", "Hide hidden files in Finder", "defaults write com.apple.finder AppleShowAllFiles -bool false && killall Finder", "", false),
            Tweak::new("  Show Path Bar", "Show path bar at bottom of Finder windows", "defaults write com.apple.finder ShowPathbar -bool true && killall Finder", "", false),
            Tweak::new("  Show Status Bar", "Show status bar at bottom of Finder windows", "defaults write com.apple.finder ShowStatusBar -bool true && killall Finder", "", false),
            Tweak::new("  Show Sidebar", "Show sidebar in Finder windows", "defaults write com.apple.finder ShowSidebar -bool true && killall Finder", "", false),
            Tweak::new("  Show Tab Bar", "Show tab bar in Finder windows", "defaults write com.apple.finder ShowTabView -bool true && killall Finder", "", false),
            Tweak::new("Finder Behavior", "Configure Finder behavior", "", "", false),
            Tweak::new("  Show All File Extensions", "Show file extensions for all files", "defaults write NSGlobalDomain AppleShowAllExtensions -bool true && killall Finder", "", false),
            Tweak::new("  Disable .DS_Store Creation", "Prevent creation of .DS_Store files", "defaults write com.apple.desktopservices DSDontWriteNetworkStores -bool true", "", false),
            Tweak::new("  Show Library Folder", "Show Library folder in user's home directory", "chflags nohidden ~/Library", "", false),
            Tweak::new("  Hide Library Folder", "Hide Library folder in user's home directory", "chflags hidden ~/Library", "", false),
            Tweak::new("  Enable Quit Option", "Enable Quit option in Finder menu", "defaults write com.apple.finder QuitMenuItem -bool true && killall Finder", "", false),
        ];

        let system_ui_tweaks = vec![
            Tweak::new("Menu Bar", "Customize menu bar appearance", "", "", false),
            Tweak::new("  Show Battery Percentage", "Show battery percentage in menu bar", "defaults write com.apple.menuextra.battery ShowPercent -string YES", "", false),
            Tweak::new("  Hide Battery Percentage", "Hide battery percentage in menu bar", "defaults write com.apple.menuextra.battery ShowPercent -string NO", "", false),
            Tweak::new("  Show Date in Menu Bar", "Show date in menu bar", "defaults write com.apple.menuextra.clock DateFormat -string 'EEE MMM d  h:mm a'", "", false),
            Tweak::new("  Show Seconds in Clock", "Show seconds in menu bar clock", "defaults write com.apple.menuextra.clock ShowSeconds -bool true", "", false),
            Tweak::new("  Hide Seconds in Clock", "Hide seconds in menu bar clock", "defaults write com.apple.menuextra.clock ShowSeconds -bool false", "", false),
            Tweak::new("Desktop & Screensaver", "Customize desktop and screensaver", "", "", false),
            Tweak::new("  Disable Screensaver", "Disable screensaver", "defaults -currentHost write com.apple.screensaver idleTime -int 0", "", false),
            Tweak::new("  Set Screensaver to 5 minutes", "Set screensaver to activate after 5 minutes", "defaults -currentHost write com.apple.screensaver idleTime -int 300", "", false),
            Tweak::new("  Set Screensaver to 10 minutes", "Set screensaver to activate after 10 minutes", "defaults -currentHost write com.apple.screensaver idleTime -int 600", "", false),
            Tweak::new("  Disable Hot Corners", "Disable hot corners", "defaults write com.apple.dock wvous-tl -int 0 && defaults write com.apple.dock wvous-tr -int 0 && defaults write com.apple.dock wvous-bl -int 0 && defaults write com.apple.dock wvous-br -int 0 && killall Dock", "", false),
            Tweak::new("Keyboard", "Customize keyboard settings", "", "", false),
            Tweak::new("  Disable Caps Lock Delay", "Remove the delay when enabling Caps Lock", "hidutil property --set '{\"CapsLockDelayOverride\":0}'", "", false),
            Tweak::new("  Set Custom Menu Bar Text", "Replace clock with custom text. You will be prompted for text.", "__PROMPT_FOR_TEXT__:defaults write com.apple.menuextra.clock DateFormat -string \"'{}'\"", "", false),
            Tweak::new("  Reset Menu Bar Clock", "Restore the default clock display", "defaults delete com.apple.menuextra.clock DateFormat", "", false),
        ];

        let security_tweaks = vec![
            Tweak::new("Gatekeeper", "Configure Gatekeeper security settings", "", "", false),
            Tweak::new("  Disable Gatekeeper", "Disable Gatekeeper (allow apps from anywhere)", "sudo spctl --master-disable", "", false),
            Tweak::new("  Enable Gatekeeper", "Enable Gatekeeper (default security)", "sudo spctl --master-enable", "", false),
            Tweak::new("  Check Gatekeeper Status", "Check current Gatekeeper status", "spctl --status", "", false),
            Tweak::new("Firewall", "Configure firewall settings", "", "", false),
            Tweak::new("  Enable Firewall", "Enable macOS firewall", "sudo /usr/libexec/ApplicationFirewall/socketfilterfw --setglobalstate on", "", false),
            Tweak::new("  Disable Firewall", "Disable macOS firewall", "sudo /usr/libexec/ApplicationFirewall/socketfilterfw --setglobalstate off", "", false),
            Tweak::new("  Check Firewall Status", "Check firewall status", "sudo /usr/libexec/ApplicationFirewall/socketfilterfw --getglobalstate", "", false),
            Tweak::new("Privacy Settings", "Configure privacy settings", "", "", false),
            Tweak::new("  Disable Location Services", "Disable location services", "sudo defaults write /var/db/locationd/Library/Preferences/ByHost/com.apple.locationd LocationServicesEnabled -int 0", "", false),
            Tweak::new("  Enable Location Services", "Enable location services", "sudo defaults write /var/db/locationd/Library/Preferences/ByHost/com.apple.locationd LocationServicesEnabled -int 1", "", false),
            Tweak::new("  Disable Analytics", "Disable analytics and diagnostics", "defaults write com.apple.AnalyticsClient AnalyticsEnabled -bool false", "", false),
            Tweak::new("  Enable Analytics", "Enable analytics and diagnostics", "defaults write com.apple.AnalyticsClient AnalyticsEnabled -bool true", "", false),
        ];

        let developer_tweaks = vec![
            Tweak::new("Developer Tools", "Install and configure developer tools", "", "", false),
            Tweak::new("  Install Xcode Command Line Tools", "Install Xcode command line tools", "xcode-select --install", "", false),
            Tweak::new("  Check Xcode Tools Status", "Check if Xcode command line tools are installed", "xcode-select -p", "", false),
            Tweak::new("  Accept Xcode License", "Accept Xcode license agreement", "sudo xcodebuild -license accept", "", false),
            Tweak::new("  Reset Xcode Path", "Reset Xcode developer directory path", "sudo xcode-select --reset", "", false),
            Tweak::new("Terminal Customization", "Customize terminal appearance", "", "", false),
            Tweak::new("  Enable Terminal Colors", "Enable colors in terminal", "defaults write com.apple.Terminal 'Default Window Settings' -string 'Pro' && defaults write com.apple.Terminal 'Startup Window Settings' -string 'Pro'", "", false),
            Tweak::new("  Set Terminal Font Size to 12", "Set terminal font size to 12", "defaults write com.apple.Terminal Pro -dict 'Font' -string 'SF Mono 12'", "", false),
            Tweak::new("  Set Terminal Font Size to 14", "Set terminal font size to 14", "defaults write com.apple.Terminal Pro -dict 'Font' -string 'SF Mono 14'", "", false),
            Tweak::new("  Set Terminal Font Size to 16", "Set terminal font size to 16", "defaults write com.apple.Terminal Pro -dict 'Font' -string 'SF Mono 16'", "", false),
            Tweak::new("  Enable Terminal Transparency", "Enable transparency in terminal", "defaults write com.apple.Terminal Pro -dict 'Transparency' -float 0.8", "", false),
            Tweak::new("Git Configuration", "Configure Git settings", "", "", false),
            Tweak::new("  Set Git Global User", "Set Git global user name and email", "git config --global user.name 'Your Name' && git config --global user.email 'your.email@example.com'", "", false),
            Tweak::new("  Configure Git Credentials", "Set up Git credential helper", "git config --global credential.helper osxkeychain", "", false),
            Tweak::new("  Set Git Default Branch", "Set default branch name to main", "git config --global init.defaultBranch main", "", false),
            Tweak::new("  Configure Git Aliases", "Set up useful Git aliases", "git config --global alias.st status && git config --global alias.co checkout && git config --global alias.br branch && git config --global alias.ci commit", "", false),
        ];

        let performance_tweaks = vec![
            Tweak::new("Animation Settings", "Configure system animations", "", "", false),
            Tweak::new("  Disable Window Animations", "Disable window animations", "defaults write NSGlobalDomain NSAutomaticWindowAnimationsEnabled -bool false", "", false),
            Tweak::new("  Enable Window Animations", "Enable window animations", "defaults write NSGlobalDomain NSAutomaticWindowAnimationsEnabled -bool true", "", false),
            Tweak::new("  Disable Dock Animations", "Disable dock animations", "defaults write com.apple.dock expose-animation-duration -float 0 && killall Dock", "", false),
            Tweak::new("  Enable Dock Animations", "Enable dock animations", "defaults write com.apple.dock expose-animation-duration -float 0.1 && killall Dock", "", false),
            Tweak::new("  Disable Menu Bar Animations", "Disable menu bar animations", "defaults write NSGlobalDomain NSWindowResizeTime -float 0.001", "", false),
            Tweak::new("  Enable Menu Bar Animations", "Enable menu bar animations", "defaults write NSGlobalDomain NSWindowResizeTime -float 0.2", "", false),
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
            Tweak::new("Wake Settings", "Configure wake behavior", "", "", false),
            Tweak::new("  Enable Wake on Network", "Enable wake on network access", "sudo systemsetup -setwakeonnetworkaccess on", "", false),
            Tweak::new("  Disable Wake on Network", "Disable wake on network access", "sudo systemsetup -setwakeonnetworkaccess off", "", false),
            Tweak::new("  Enable Wake on Modem", "Enable wake on modem ring", "sudo systemsetup -setwakeonmodem on", "", false),
            Tweak::new("  Disable Wake on Modem", "Disable wake on modem ring", "sudo systemsetup -setwakeonmodem off", "", false),
        ];

        let network_tweaks = vec![
            Tweak::new("DNS Management", "Manage DNS settings", "", "", false),
            Tweak::new("  Flush DNS Cache", "Removes all entries from the DNS cache", "sudo dscacheutil -flushcache; sudo killall -HUP mDNSResponder", "", false),
            Tweak::new("  Set DNS to Google", "Set DNS servers to Google (8.8.8.8, 8.8.4.4)", "networksetup -setdnsservers Wi-Fi 8.8.8.8 8.8.4.4", "", false),
            Tweak::new("  Set DNS to Cloudflare", "Set DNS servers to Cloudflare (1.1.1.1, 1.0.0.1)", "networksetup -setdnsservers Wi-Fi 1.1.1.1 1.0.0.1", "", false),
            Tweak::new("  Reset DNS to DHCP", "Reset DNS to use DHCP", "networksetup -setdnsservers Wi-Fi empty", "", false),
            Tweak::new("Network Interfaces", "Configure network interfaces", "", "", false),
            Tweak::new("  Enable Wi-Fi", "Enable Wi-Fi interface", "networksetup -setairportpower en0 on", "", false),
            Tweak::new("  Disable Wi-Fi", "Disable Wi-Fi interface", "networksetup -setairportpower en0 off", "", false),
            Tweak::new("  Enable Bluetooth", "Enable Bluetooth", "sudo pkill bluetoothd", "", false),
            Tweak::new("  Disable Bluetooth", "Disable Bluetooth", "sudo pkill bluetoothd", "", false),
            Tweak::new("  Show Network Info", "Show detailed network information", "networksetup -listallnetworkservices && echo '---' && ifconfig", "", false),
        ];

        let optimization_tweaks = vec![
            Tweak::new("Clean Up Caches", "Remove temporary cache files", "", "", false),
            Tweak::new("  Clear User Cache (destructive)", "Removes all files from ~/Library/Caches", "rm -rf ~/Library/Caches/*", "", false),
            Tweak::new("  Clear System Cache (destructive)", "Removes all files from /Library/Caches", "sudo rm -rf /Library/Caches/*", "", false),
            Tweak::new("  Clear Launch Services Cache", "Clear Launch Services cache", "sudo rm -rf /System/Library/Caches/com.apple.LaunchServices-*.csstore", "", false),
            Tweak::new("  Clear Xcode Derived Data", "Clear Xcode derived data (if Xcode is installed)", "rm -rf ~/Library/Developer/Xcode/DerivedData", "", false),

            Tweak::new("Organize Desktop", "Move files from Desktop to organized folders", "", "", false),
            Tweak::new("  Move screenshots to Pictures folder", "Finds all screenshots on Desktop and moves them to ~/Pictures/Screenshots", "mkdir -p ~/Pictures/Screenshots && find ~/Desktop -maxdepth 1 \\( -name 'Screen Shot*.png' -o -name 'Screenshot*.png' \\) -exec mv -n {} ~/Pictures/Screenshots/ \\;", "", false),
            Tweak::new("  Move project folders to ~/Developer", "Moves folders with .git, .gitignore, or source code", "zsh scripts/organize_projects.sh", "", false),
            Tweak::new("  Move images to ~/Pictures", "Moves common image files from Desktop to Pictures", "find ~/Desktop -maxdepth 1 -type f \\( -iname '*.png' -o -iname '*.jpg' -o -iname '*.jpeg' -o -iname '*.gif' \\) -exec mv -n {} ~/Pictures/ \\;", "", false),
            Tweak::new("  Move videos to ~/Movies", "Moves common video files from Desktop to Movies", "find ~/Desktop -maxdepth 1 -type f \\( -iname '*.mov' -o -iname '*.mp4' \\) -exec mv -n {} ~/Movies/ \\;", "", false),
            Tweak::new("  Move documents to ~/Documents", "Moves common document files from Desktop to Documents", "find ~/Desktop -maxdepth 1 -type f \\( -iname '*.pdf' -o -iname '*.docx' \\) -exec mv -n {} ~/Documents/ \\;", "", false),

            Tweak::new("Find Large Files", "Identify large files to free up space", "", "", false),
            Tweak::new("  List 10 largest files in Home", "Shows a list of the 10 biggest files in your home directory.", "echo 'Large files in home directory:' && ls -lah ~ | grep -v '^d' | sort -k5 -hr | head -n 10", "", false),
            Tweak::new("  Find files larger than 100MB", "Find all files larger than 100MB in home directory", "find ~ -type f -size +100M -exec ls -lh {} \\; 2>/dev/null", "", false),
            Tweak::new("  Find files larger than 1GB", "Find all files larger than 1GB in home directory", "find ~ -type f -size +1G -exec ls -lh {} \\; 2>/dev/null", "", false),
            
            Tweak::new("System Maintenance", "Perform system maintenance tasks", "", "", false),
            Tweak::new("  Repair Disk Permissions", "Repair disk permissions", "sudo diskutil resetUserPermissions / `id -u`", "", false),
            Tweak::new("  Clear System Logs", "Clear system logs (requires admin)", "sudo rm -rf /var/log/*.log", "", false),
            Tweak::new("  Clear User Logs", "Clear user logs", "rm -rf ~/Library/Logs/*", "", false),
            Tweak::new("  Rebuild Spotlight Index", "Rebuild Spotlight search index", "sudo mdutil -E /", "", false),
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
            Tweak::new("  Upgrade Specific Package", "Upgrade a specific package", "brew upgrade [package_name]", "", false),
            Tweak::new("  Install Common Dev Tools", "Install common development tools", "brew install git node python3 rust go", "", false),
            
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
            Tweak::new("  Sokoban Game", "Start the Sokoban puzzle game", "__SOKOBAN_GAME__", "", false),
        ];

        let utilities_tweaks = vec![
            Tweak::new("System Information", "Get detailed system information", "", "", false),
            Tweak::new("  Show Disk Usage", "Show disk usage information", "df -h", "", false),
            Tweak::new("  Show Memory Usage", "Show memory usage information", "vm_stat", "", false),
            Tweak::new("  Show CPU Info", "Show CPU information", "sysctl -n machdep.cpu.brand_string", "", false),
            Tweak::new("  Show Network Interfaces", "Show network interface information", "ifconfig", "", false),
            Tweak::new("  Show Running Processes", "Show top running processes", "ps aux | head -20", "", false),
            Tweak::new("File & Directory", "Useful file and directory operations", "", "", false),
            Tweak::new("  Count Files in Directory", "Count files in current directory", "ls -1 | wc -l", "", false),
            Tweak::new("  Find Empty Files", "Find empty files in current directory", "find . -type f -empty", "", false),
            Tweak::new("  Find Large Files (>100MB)", "Find files larger than 100MB in current directory", "find . -type f -size +100M -exec ls -lh {} \\;", "", false),
            Tweak::new("Maintenance & Network", "System maintenance and network utilities", "", "", false),
            Tweak::new("  Flush DNS Cache", "Clear DNS cache", "sudo dscacheutil -flushcache; sudo killall -HUP mDNSResponder", "", false),
            Tweak::new("  Clear Launch Services Cache", "Clear Launch Services cache", "sudo rm -rf /System/Library/Caches/com.apple.LaunchServices-*.csstore", "", false),
            Tweak::new("  Rebuild Spotlight Index", "Rebuild Spotlight search index", "sudo mdutil -E /", "", false),
            Tweak::new("  Repair Disk Permissions", "Repair disk permissions", "sudo diskutil resetUserPermissions / `id -u`", "", false),
            Tweak::new("  Show System Logs", "Show recent system logs", "log show --last 1h | head -50", "", false),
            Tweak::new("  Test Internet Connection", "Test internet connectivity", "ping -c 3 8.8.8.8", "", false),
            Tweak::new("  Show Network Speed", "Show current network interface speeds", "top -l 1 | grep \"Networks:\"", "", false),
            Tweak::new("  Show Active Connections", "Show active network connections", "netstat -an | grep ESTABLISHED | head -10", "", false),
            Tweak::new("  Test DNS Resolution", "Test DNS resolution", "nslookup google.com", "", false),
        ];

        let categories = vec![
            TopLevelCategory::new("Dock", "Customize macOS Dock settings", dock_tweaks),
            TopLevelCategory::new("Finder", "Customize Finder appearance and behavior", finder_tweaks),
            TopLevelCategory::new("System UI", "Customize system user interface", system_ui_tweaks),
            TopLevelCategory::new("Security", "Configure security and privacy settings", security_tweaks),
            TopLevelCategory::new("Developer", "Developer tools and configurations", developer_tweaks),
            TopLevelCategory::new("Performance", "Optimize system performance", performance_tweaks),
            TopLevelCategory::new("Animated Wallpapers", "Enable animated wallpapers", animated_wallpapers_tweaks),
            TopLevelCategory::new("Power Management", "Configure sleep and power settings", power_management_tweaks),
            TopLevelCategory::new("Networking", "Configure network settings", network_tweaks),
            TopLevelCategory::new("Optimization", "Apply system performance tweaks", optimization_tweaks),
            TopLevelCategory::new("Brew Management", "Manage Homebrew package manager", brew_tweaks),
            TopLevelCategory::new("About", "Application information and system details", about_tweaks),
            TopLevelCategory::new("Utilities", "Useful system utilities", utilities_tweaks),
        ];

        let mut category_list_state = ListState::default();
        category_list_state.select(Some(0));

        App {
            view_level: 0,
            selected_indices: [0, 0],
            category_list_state,
            tweak_list_state: ListState::default(),
            viewing_sub_category: None,
            should_quit: false,
            categories,
            applied_tweaks: Vec::new(),
            status_message: None,
            status_timer: 0,
            pending_destructive_command: None,
            confirmation_message: None,
            text_input_prompt: None,
            text_input_command_template: None,
            input_buffer: String::new(),
            fullscreen_output: None,
            fullscreen_output_scroll: 0,
            config,
            fullscreen_list: None,
            fullscreen_list_state: ListState::default(),
            fullscreen_list_title: String::new(),
            sokoban_game: None,
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
        if count == 0 {
            return;
        }

        let (index, state) = if self.view_level == 0 {
            (&mut self.selected_indices[0], &mut self.category_list_state)
        } else {
            (&mut self.selected_indices[1], &mut self.tweak_list_state)
        };
        
        let new_index = (*index + 1) % count;
        *index = new_index;
        state.select(Some(new_index));
    }
    
    pub fn previous_item(&mut self) {
        let count = self.get_current_list_items().len();
        if count == 0 {
            return;
        }

        let (index, state) = if self.view_level == 0 {
            (&mut self.selected_indices[0], &mut self.category_list_state)
        } else {
            (&mut self.selected_indices[1], &mut self.tweak_list_state)
        };

        let new_index = if *index == 0 { count - 1 } else { *index - 1 };
        *index = new_index;
        state.select(Some(new_index));
    }
    
    pub fn handle_right_key(&mut self) {
        match self.view_level {
            0 => { // From top-level to sub-categories
                if !self.categories[self.selected_indices[0]].tweaks.is_empty() {
                    self.view_level = 1;
                    self.selected_indices[1] = 0;
                    self.tweak_list_state = ListState::default();
                    self.tweak_list_state.select(Some(0));
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
                            self.tweak_list_state = ListState::default();
                            self.tweak_list_state.select(Some(0));
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
                    self.tweak_list_state = ListState::default();
                    self.tweak_list_state.select(Some(0));
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
                    self.fullscreen_output_scroll = 0;
                    return Ok(());
                }
                if tweak.enable_command == "__SOKOBAN_GAME__" {
                    self.sokoban_game = Some(SokobanGame::new());
                    return Ok(());
                }
                if tweak.enable_command.starts_with("__PROMPT_FOR_TEXT__:") {
                    if let Some(template) = tweak.enable_command.strip_prefix("__PROMPT_FOR_TEXT__:") {
                        self.text_input_prompt = Some(format!("Enter text for: {}", tweak.name.trim()));
                        self.text_input_command_template = Some(template.to_string());
                        self.input_buffer.clear();
                        return Ok(());
                    }
                }
                if tweak.enable_command == "__CHECK_BREW__" {
                    let message = if utils::check_command_exists("brew") {
                        "Homebrew is installed and available in your PATH."
                    } else {
                        "Homebrew is not installed or not in your PATH."
                    };
                    self.fullscreen_output = Some(message.to_string());
                    self.fullscreen_output_scroll = 0;
                    return Ok(());
                }
                if tweak.enable_command == "__LIST_INSTALLED__" {
                    match utils::execute_command("brew list", false) {
                        Ok(output) => {
                            let packages: Vec<String> = output.lines().filter(|l| !l.trim().is_empty()).map(String::from).collect();
                            if packages.is_empty() {
                                self.fullscreen_output = Some("No installed Homebrew packages found.".to_string());
                                self.fullscreen_output_scroll = 0;
                            } else {
                                self.fullscreen_list = Some(packages);
                                self.fullscreen_list_state.select(Some(0));
                                self.fullscreen_list_title = "Installed Packages (Press Enter for info)".to_string();
                            }
                        }
                        Err(e) => {
                            self.fullscreen_output = Some(format!("Error fetching installed packages: {}", e));
                            self.fullscreen_output_scroll = 0;
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
                                self.fullscreen_output_scroll = 0;
                            } else {
                                self.fullscreen_list = Some(packages);
                                self.fullscreen_list_state.select(Some(0));
                                self.fullscreen_list_title = "Outdated Packages (Press Enter to upgrade)".to_string();
                            }
                        }
                        Err(e) => {
                            self.fullscreen_output = Some(format!("Error fetching outdated packages: {}", e));
                            self.fullscreen_output_scroll = 0;
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
                let is_info_command = tweak_name.contains("List") || tweak_name.contains("Show") || tweak_name.contains("About") || tweak_name.contains("Version") || tweak_name.contains("Dependencies") || tweak_name.contains("System Information") || tweak_name.contains("Count") || tweak_name.contains("Find");
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
                    if !is_info_command && !can_run_multiple && !self.applied_tweaks.contains(&tweak_name) {
                        self.applied_tweaks.push(tweak_name.clone());
                    }
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
                                self.fullscreen_output_scroll = 0;
                            } else {
                                if !can_run_multiple && !self.applied_tweaks.contains(&tweak_name) {
                                    self.applied_tweaks.push(tweak_name.clone());
                                }
                                if output.trim().is_empty() {
                                    self.fullscreen_output = Some(format!("'{}' executed successfully with no output.", tweak_name.trim()));
                                    self.fullscreen_output_scroll = 0;
                                } else {
                                    self.status_message = Some(format!("Successfully applied: {}", tweak_name.trim()));
                                    self.status_timer = 50;
                                }
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