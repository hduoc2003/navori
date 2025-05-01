use std::fs::File;
use std::io::BufReader;
use std::str::FromStr;
use crate::config::{AppConfig, StatInfo, VerifyMerkleStat};
use aptos_sdk::move_types::u256::U256;
use aptos_sdk::move_types::value::MoveValue;
use serde::{Deserialize, Serialize};

use crate::utils::{build_and_submit, get_event_from_transaction, str_to_u256, str_to_u64};
use aptos_sdk::move_types::identifier::Identifier;
use aptos_sdk::move_types::language_storage::ModuleId;
use aptos_sdk::move_types::value::serialize_values;
use aptos_sdk::rest_client::aptos_api_types::{Event, MoveType};
use aptos_sdk::types::transaction::{EntryFunction, TransactionPayload};
use log::{info};
use tokio::time::Instant;

pub async fn verify_merkle(
    config: &AppConfig,
    verify_merkle_input: VerifyMerkleTransactionInput,
) -> anyhow::Result<VerifyMerkleStat> {
    let (input_verify_merkle, input_register_fact_merkle, vms_stat) =
        verify_merkle_statement(config, verify_merkle_input).await?;

    let mv_stat = merkle_verifier(config, &input_verify_merkle).await?;

    let rfm_stat = register_fact_merkle(config, &input_register_fact_merkle).await?;

    Ok(VerifyMerkleStat {
        msc_verify_merkle: vms_stat,
        mv_verify_merkle: mv_stat,
        register_fact_verify_merkle: rfm_stat,
    })
}

pub async fn verify_merkle_statement(
    config: &AppConfig,
    data: VerifyMerkleTransactionInput,
) -> anyhow::Result<(VerifyMerkle, RegisterFactVerifyMerkle, StatInfo)> {
    let t = Instant::now();
    let payload = TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(
            config.verifier_address,
            Identifier::new("merkle_statement_contract")?,
        ),
        Identifier::new("verify_merkle")?,
        vec![],
        serialize_values(&vec![
            data.merkle_view,
            data.initial_merkle_queue,
            data.height,
            data.expected_root,
        ]),
    ));

    let transaction = build_and_submit(&config.client, payload, &config.account, config.chain_id, Some(5), None).await?;
    let transaction_info = transaction.transaction_info()?;
    info!(
        "verify_merkle_statement finished: id={}; hash={}; gas={}",
        transaction_info.version,
        transaction_info.hash.to_string(),
        transaction_info.gas_used
    );

    let verify_merkle_event_type = MoveType::from_str(&format!(
        "{}::merkle_statement_contract::VerifyMerkle",
        config.verifier_address
    ))?;
    let verify_merkle_data =
        get_event_from_transaction(&transaction, verify_merkle_event_type)?.clone();

    let register_fact_event_type = MoveType::from_str(&format!(
        "{}::merkle_statement_contract::RegisterFactVerifyMerkle",
        config.verifier_address
    ))?;
    let register_fact_data =
        get_event_from_transaction(&transaction, register_fact_event_type)?.clone();

    let input_verify_merkle: VerifyMerkle = verify_merkle_data.try_into()?;
    let input_register_fact_merkle: RegisterFactVerifyMerkle = register_fact_data.try_into()?;

    let stat = StatInfo {
        time: t.elapsed().as_secs_f32(),
        gas_used: transaction_info.gas_used.0
    };
    Ok((input_verify_merkle, input_register_fact_merkle, stat))
}

pub async fn merkle_verifier(config: &AppConfig, data: &VerifyMerkle) -> anyhow::Result<StatInfo> {
    let t = Instant::now();
    let params = serialize_values(&vec![
        MoveValue::U64(data.channel_ptr),
        MoveValue::U64(data.merkle_queue_ptr),
        MoveValue::U256(data.expected_root),
        MoveValue::U64(data.n_queries),
    ]);
    let payload = TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(config.verifier_address, Identifier::new("merkle_verifier")?),
        Identifier::new("verify_merkle")?,
        vec![],
        params,
    ));

    let transaction = build_and_submit(&config.client, payload, &config.account, config.chain_id, Some(5), None).await?;
    let transaction_info = transaction.transaction_info()?;
    info!(
        "verify_merkle finished: id={}; hash={}; gas={}",
        transaction_info.version,
        transaction_info.hash.to_string(),
        transaction_info.gas_used
    );
    let stat = StatInfo {
        time: t.elapsed().as_secs_f32(),
        gas_used: transaction_info.gas_used.0
    };
    Ok(stat)
}

pub async fn register_fact_merkle(
    config: &AppConfig,
    data: &RegisterFactVerifyMerkle,
) -> anyhow::Result<StatInfo> {
    let t = Instant::now();
    let payload = TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(
            config.verifier_address,
            Identifier::new("merkle_statement_contract")?,
        ),
        Identifier::new("register_fact_verify_merkle")?,
        vec![],
        serialize_values(&vec![
            MoveValue::U64(data.channel_ptr),
            MoveValue::U64(data.data_to_hash_ptr),
            MoveValue::U64(data.n_queries),
            MoveValue::U256(data.res_root),
        ]),
    ));
    let transaction = build_and_submit(&config.client, payload, &config.account, config.chain_id, Some(5), None).await?;
    let transaction_info = transaction.transaction_info()?;
    info!(
        "transaction register_fact_verify_merkle = {:#?}",
        transaction.transaction_info()?.hash.to_string()
    );
    let stat = StatInfo {
        time: t.elapsed().as_secs_f32(),
        gas_used: transaction_info.gas_used.0
    };
    Ok(stat)
}

pub fn sample_verify_merkle_input(index: isize) -> anyhow::Result<VerifyMerkleTransactionInput> {
    let file_path = format!("./data/merkle_verify/merkle_verify_{}.json", index);
    let input_file = File::open(file_path)?;
    let reader = BufReader::new(input_file);
    let merkle_verify_input: MerkleVerifyInput = serde_json::from_reader(reader)?;

    let mut merkle_view_vec = vec![];
    for i in 0..merkle_verify_input.merkle_view.len() {
        merkle_view_vec.push(MoveValue::U256(U256::from_str(
            &merkle_verify_input.merkle_view[i].clone(),
        )?));
    }
    let merkle_view = MoveValue::Vector(merkle_view_vec);

    let mut initial_merkle_queue_vec = vec![];
    for i in 0..merkle_verify_input.initial_merkle_queue.len() {
        initial_merkle_queue_vec.push(MoveValue::U256(U256::from_str(
            &merkle_verify_input.initial_merkle_queue[i],
        )?));
    }
    let initial_merkle_queue = MoveValue::Vector(initial_merkle_queue_vec);

    let height = MoveValue::U64(u64::from_str(&merkle_verify_input.height.clone())?);
    let expected_root =
        MoveValue::U256(U256::from_str(&merkle_verify_input.expected_root.clone())?);
    Ok(VerifyMerkleTransactionInput {
        merkle_view,
        initial_merkle_queue,
        height,
        expected_root,
    })
}

#[derive(Serialize, Debug)]
pub struct VerifyMerkleTransactionInput {
    pub merkle_view: MoveValue,
    pub initial_merkle_queue: MoveValue,
    pub height: MoveValue,
    pub expected_root: MoveValue,
}

#[derive(Debug)]
pub struct VerifyMerkle {
    pub channel_ptr: u64,
    pub merkle_queue_ptr: u64,
    pub expected_root: U256,
    pub n_queries: u64,
}

#[derive(Debug)]
pub struct RegisterFactVerifyMerkle {
    pub channel_ptr: u64,
    pub data_to_hash_ptr: u64,
    pub n_queries: u64,
    pub res_root: U256,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MerkleVerifyInput {
    pub merkle_view: Vec<String>,
    pub initial_merkle_queue: Vec<String>,
    pub height: String,
    pub expected_root: String,
}

impl TryInto<VerifyMerkle> for Event {
    type Error = anyhow::Error;

    fn try_into(self) -> anyhow::Result<VerifyMerkle> {
        Ok(VerifyMerkle {
            channel_ptr: str_to_u64(self.data.get("channel_ptr").unwrap().as_str().unwrap())?,
            merkle_queue_ptr: str_to_u64(
                self.data.get("merkle_queue_ptr").unwrap().as_str().unwrap(),
            )?,
            expected_root: str_to_u256(self.data.get("expected_root").unwrap().as_str().unwrap())?,
            n_queries: str_to_u64(self.data.get("n_queries").unwrap().as_str().unwrap())?,
        })
    }
}

impl TryInto<RegisterFactVerifyMerkle> for Event {
    type Error = anyhow::Error;

    fn try_into(self) -> anyhow::Result<RegisterFactVerifyMerkle> {
        Ok(RegisterFactVerifyMerkle {
            channel_ptr: str_to_u64(self.data.get("channel_ptr").unwrap().as_str().unwrap())?,
            data_to_hash_ptr: str_to_u64(
                self.data.get("data_to_hash_ptr").unwrap().as_str().unwrap(),
            )?,
            n_queries: str_to_u64(self.data.get("n_queries").unwrap().as_str().unwrap())?,
            res_root: str_to_u256(self.data.get("res_root").unwrap().as_str().unwrap())?,
        })
    }
}
