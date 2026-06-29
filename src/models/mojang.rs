use serde::Deserialize;
use std::collections::HashMap;

/// Manifest of all Minecraft versions
#[derive(Deserialize, Debug)]
pub struct VersionManifest {
    pub versions: Vec<VersionEntry>,
}

/// A version entry in the Mojang manifest
#[derive(Deserialize, Debug)]
pub struct VersionEntry {
    pub id: String,
    pub url: String,
}

/// Package description for a specific Minecraft version
#[derive(Deserialize, Debug)]
pub struct VersionPackage {
    pub downloads: VersionDownloads,
    pub libraries: Vec<MojangLibrary>,
}

/// Download details of the client and server jars
#[derive(Deserialize, Debug)]
pub struct VersionDownloads {
    pub client: VersionArtifact,
    pub server: Option<VersionArtifact>,
}

/// Raw artifact details
#[derive(Deserialize, Debug)]
pub struct VersionArtifact {
    pub url: String,
}

/// A dependency library required by Minecraft client/server
#[derive(Deserialize, Debug)]
pub struct MojangLibrary {
    pub name: String,
    pub downloads: MojangDownloads,
    pub natives: Option<HashMap<String, String>>,
}

/// Downloads mappings of dependencies
#[derive(Deserialize, Debug)]
pub struct MojangDownloads {
    pub artifact: Option<MojangArtifact>,
    pub classifiers: Option<HashMap<String, MojangArtifact>>,
}

/// Binary location and path of dependency files
#[derive(Deserialize, Debug)]
pub struct MojangArtifact {
    pub url: String,
    pub path: String,
}

/// Fabric loader metadata
#[derive(Deserialize, Debug)]
pub struct FabricLoaderEntry {
    pub version: String,
}

/// Fabric installer metadata
#[derive(Deserialize, Debug)]
pub struct FabricInstallerEntry {
    pub version: String,
}

/// Fabric download profile library listing
#[derive(Deserialize, Debug)]
pub struct FabricProfileResponse {
    pub libraries: Vec<FabricProfileLibrary>,
}

/// Fabric profile dependency representation
#[derive(Deserialize, Debug)]
pub struct FabricProfileLibrary {
    pub name: String,
    pub url: String,
}

/// PaperMC builds list API model
#[derive(Deserialize, Debug)]
pub struct PaperVersionResponse {
    pub builds: Vec<u32>,
}

/// Ping players count info
#[derive(serde::Deserialize, Debug, Clone)]
pub struct PingPlayers {
    pub max: u32,
    pub online: u32,
}

/// Minecraft server ping response properties
#[derive(serde::Deserialize, Debug, Clone)]
pub struct MinecraftPingResponse {
    pub players: Option<PingPlayers>,
    pub description: serde_json::Value,
}
