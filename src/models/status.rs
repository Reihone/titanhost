/// Main navigation tabs in the GUI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Launch,
    Modding,
    ServerSettings,
    Backups,
}

/// Filter for server console logs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConsoleFilter {
    #[default]
    All,
    Info,
    Warn,
    Error,
    Events,
}

/// Subtabs inside the Modding section
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModdingSubTab {
    #[default]
    CoresAndVersions,
    Mods,
    Plugins,
    Dependencies,
}

/// Minecraft server ping response properties
#[derive(Debug, Clone)]
pub struct ServerStatus {
    pub is_online: bool,
    pub motd: String,
    pub players_online: u32,
    pub players_max: u32,
    pub ping_ms: u128,
}
