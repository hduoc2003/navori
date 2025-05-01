use aptos_sdk::rest_client::Client;
use aptos_sdk::types::account_address::AccountAddress;
use aptos_sdk::types::chain_id::ChainId;
use aptos_sdk::types::LocalAccount;
use config::{Config as ConfigLoader, File, FileFormat};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use crate::verify_fri::{ComputeNextLayer, RegisterFactVerifyFri};

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub private_key: String,
    pub node_url: String,
    pub verifier_address: String,
    pub chain_id: String,
}

impl Config {
    pub fn from_path(path: &str) -> anyhow::Result<Config> {
        let content = ConfigLoader::builder()
            .add_source(File::new(path, FileFormat::Toml))
            .build()?;
        let args: Config = content.try_deserialize()?;
        Ok(args)
    }
}

pub struct AppConfig {
    pub client: Arc<Client>,
    pub account: Arc<LocalAccount>,
    pub verifier_address: AccountAddress,
    pub chain_id: ChainId,
}

#[derive(Default, Debug, Serialize)]
pub struct GlobalStat {
    pub verify_merkle: Vec<VerifyMerkleStat>,
    pub verify_fri: Vec<VerifyFriStat>,
    pub rcmp: Vec<StatInfo>,
    pub vpar: VparStat,
}

#[derive(Default, Debug, Serialize)]
pub struct StatInfo {
    pub time: f32,
    pub gas_used: u64
}

#[derive(Default, Debug, Serialize)]
pub struct VerifyMerkleStat {
    pub msc_verify_merkle: StatInfo,
    pub mv_verify_merkle: StatInfo,
    pub register_fact_verify_merkle: StatInfo
}

#[derive(Default, Debug, Serialize)]
pub struct VerifyFriStat {
    pub verify_fri: StatInfo,
    pub init_fri_group: StatInfo,
    pub compute_next_layer: StatInfo,
    pub verify_merkle: StatInfo,
    pub register_fact_verify_fri: StatInfo
}

#[derive(Default, Debug, Serialize)]
pub struct VparStat {
    pub prepush_task_metadata: StatInfo,
    pub prepush_data: StatInfo,
    pub vpar: Vec<StatInfo>,
    pub reset_data: StatInfo,
}
impl From<Config> for AppConfig {
    fn from(config: Config) -> Self {
        let client = Arc::new(Client::new(config.node_url.parse().unwrap()));
        let account = Arc::new(LocalAccount::from_private_key(&config.private_key, 0).unwrap());
        let verifier_address = config
            .verifier_address
            .parse()
            .expect("Invalid verifier address");
        AppConfig {
            client,
            account,
            verifier_address,
            chain_id: ChainId::from_str(&config.chain_id).expect("Invalid chain id"),
        }
    }
}
