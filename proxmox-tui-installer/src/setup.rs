use std::{cmp, collections::HashMap, fmt, fs::File, io::BufReader, net::IpAddr, path::Path};

use serde::{ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    options::{BtrfsRaidLevel, Disk, FsType, InstallerOptions, ZfsRaidLevel},
    utils::CidrAddress,
};

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProxmoxProduct {
    PVE,
    PBS,
    PMG,
}

#[derive(Clone, Deserialize)]
pub struct ProductConfig {
    pub fullname: String,
    pub product: ProxmoxProduct,
    #[serde(deserialize_with = "deserialize_bool_from_int")]
    pub enable_btrfs: bool,
}

#[derive(Clone, Deserialize)]
pub struct IsoInfo {
    pub release: String,
    pub isorelease: String,
}

#[derive(Clone, Deserialize)]
pub struct SetupInfo {
    #[serde(rename = "product-cfg")]
    pub config: ProductConfig,
    #[serde(rename = "iso-info")]
    pub iso_info: IsoInfo,
}

#[derive(Clone, Deserialize)]
pub struct CountryInfo {
    pub name: String,
    #[serde(default)]
    pub zone: String,
    pub kmap: String,
}

#[derive(Clone, Deserialize, Eq, PartialEq)]
pub struct KeyboardMapping {
    pub name: String,
    #[serde(rename = "kvm")]
    pub id: String,
    #[serde(rename = "x11")]
    pub xkb_layout: String,
    #[serde(rename = "x11var")]
    pub xkb_variant: String,
}

impl cmp::PartialOrd for KeyboardMapping {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.name.partial_cmp(&other.name)
    }
}

impl cmp::Ord for KeyboardMapping {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.name.cmp(&other.name)
    }
}

#[derive(Clone, Deserialize)]
pub struct LocaleInfo {
    #[serde(deserialize_with = "deserialize_cczones_map")]
    pub cczones: HashMap<String, Vec<String>>,
    #[serde(rename = "country")]
    pub countries: HashMap<String, CountryInfo>,
    pub kmap: HashMap<String, KeyboardMapping>,
}

#[derive(Serialize)]
pub struct InstallConfig {
    #[serde(serialize_with = "serialize_target_disk_list")]
    target_hd: Vec<Disk>,
    #[serde(serialize_with = "serialize_fstype")]
    target_fs: FsType,
    country: String,
    timezone: String,
    keymap: String,
    mailto: String,
    password: String,
    interface: String,
    hostname: String,
    domain: String,
    ip: IpAddr,
    netmask: String,
    #[serde(serialize_with = "serialize_as_display")]
    cidr: CidrAddress,
    gateway: IpAddr,
    dnsserver: IpAddr,
}

impl From<InstallerOptions> for InstallConfig {
    fn from(options: InstallerOptions) -> Self {
        Self {
            target_hd: options.bootdisk.disks,
            target_fs: options.bootdisk.fstype,
            country: options.timezone.country,
            timezone: options.timezone.timezone,
            keymap: options.timezone.kb_layout,
            mailto: options.password.email,
            password: options.password.root_password,
            interface: options.network.ifname,
            hostname: options.network.fqdn.host().to_owned(),
            domain: options.network.fqdn.domain().to_owned(),
            ip: options.network.address.addr(),
            netmask: options.network.address.mask().to_string(),
            cidr: options.network.address,
            gateway: options.network.gateway,
            dnsserver: options.network.dns_server,
        }
    }
}

pub fn read_json<T: for<'de> Deserialize<'de>, P: AsRef<Path>>(path: P) -> Result<T, String> {
    let file = File::open(path).map_err(|err| err.to_string())?;
    let reader = BufReader::new(file);

    serde_json::from_reader(reader).map_err(|err| format!("failed to parse JSON: {err}"))
}

fn deserialize_bool_from_int<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let val: u32 = Deserialize::deserialize(deserializer)?;
    Ok(val != 0)
}

fn deserialize_cczones_map<'de, D>(
    deserializer: D,
) -> Result<HashMap<String, Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    let map: HashMap<String, HashMap<String, u32>> = Deserialize::deserialize(deserializer)?;

    let mut result = HashMap::new();
    for (cc, list) in map.into_iter() {
        result.insert(cc, list.into_keys().collect());
    }

    Ok(result)
}

fn serialize_target_disk_list<S>(value: &[Disk], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut seq = serializer.serialize_seq(Some(value.len()))?;
    for disk in value {
        seq.serialize_element(&disk.path)?;
    }
    seq.end()
}

fn serialize_fstype<S>(value: &FsType, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    use FsType::*;
    let value = match value {
        // proxinstall::$fssetup
        Ext4 => "ext4",
        Xfs => "xfs",
        // proxinstall::get_zfs_raid_setup()
        Zfs(ZfsRaidLevel::Single) => "zfs (RAID0)",
        Zfs(ZfsRaidLevel::Mirror) => "zfs (RAID1)",
        Zfs(ZfsRaidLevel::Raid10) => "zfs (RAID10)",
        Zfs(ZfsRaidLevel::RaidZ) => "zfs (RAIDZ-1)",
        Zfs(ZfsRaidLevel::RaidZ2) => "zfs (RAIDZ-2)",
        Zfs(ZfsRaidLevel::RaidZ3) => "zfs (RAIDZ-3)",
        // proxinstall::get_btrfs_raid_setup()
        Btrfs(BtrfsRaidLevel::Single) => "btrfs (RAID0)",
        Btrfs(BtrfsRaidLevel::Mirror) => "btrfs (RAID1)",
        Btrfs(BtrfsRaidLevel::Raid10) => "btrfs (RAID10)",
    };

    serializer.collect_str(value)
}

fn serialize_as_display<S, T>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: fmt::Display,
{
    serializer.collect_str(value)
}
