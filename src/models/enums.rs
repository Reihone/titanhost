use serde::{Deserialize, Serialize};

/// Supported mod loaders for Minecraft client/server instances
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum ModLoader {
    Vanilla,
    #[default]
    Forge,
    Fabric,
    NeoForge,
}

/// Supported Minecraft server core types
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum ServerCoreType {
    Vanilla,
    #[default]
    Paper,
    Forge,
    Fabric,
    NeoForge,
}
