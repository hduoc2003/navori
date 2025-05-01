use std::fs::File;
use std::io::BufReader;

use aptos_sdk::rest_client::aptos_api_types::MoveType;
use serde::Deserialize;
use std::str::FromStr;

use crate::config::{AppConfig, StatInfo, VerifyFriStat};
use crate::utils::{build_and_submit, get_event_from_transaction, str_to_u256, str_to_u64};

pub async fn verify_fri(
    config: &AppConfig,
    fri_verify_input: FriVerifyInput,
    proof: MoveValue,
    fri_queue: MoveValue,
    evaluation_point: MoveValue,
    fri_step_size: MoveValue,
    expected_root: MoveValue,
) -> anyhow::Result<VerifyFriStat> {
    let verify_merkle_input = VerifyFriTransactionInput {
        proof,
        fri_queue,
        evaluation_point,
        fri_step_size,
        expected_root,
    };

    let (input_init, input_compute, input_register, fs_stat) =
        fri_statement(config, verify_merkle_input.clone()).await?;

    let ifg_stat = init_fri_group(config, input_init).await?;

    let cnl_stat = compute_next_layer(config, &input_compute).await?;

    let input_verify_merkle: VerifyMerkle = VerifyMerkle {
        channel_ptr: input_compute.channel_ptr,
        merkle_queue_ptr: input_compute.merkle_queue_ptr,
        expected_root: U256::from_str(&fri_verify_input.expected_root)?,
        n_queries: input_compute.n_queries,
    };

    let mv_stat = merkle_verifier(config, &input_verify_merkle).await?;

    let rff_stat = register_fact_fri(config, input_register, input_compute.n_queries).await?;

    Ok(VerifyFriStat {
        verify_fri: fs_stat,
        init_fri_group: ifg_stat,
        compute_next_layer: cnl_stat,
        verify_merkle: mv_stat,
        register_fact_verify_fri: rff_stat,
    })
}

pub async fn fri_statement(
    config: &AppConfig,
    data: VerifyFriTransactionInput,
) -> anyhow::Result<(
    InitFriGroup,
    ComputeNextLayer,
    RegisterFactVerifyFri,
    StatInfo,
)> {
    let t = Instant::now();
    let payload = TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(
            config.verifier_address,
            Identifier::new("fri_statement_contract")?,
        ),
        Identifier::new("verify_fri")?,
        vec![],
        serialize_values(&vec![
            data.proof,
            data.fri_queue,
            data.evaluation_point,
            data.fri_step_size,
            data.expected_root,
        ]),
    ));
    let transaction = build_and_submit(
        &config.client,
        payload,
        &config.account,
        config.chain_id,
        Some(5),
        None,
    )
    .await?;
    let transaction_info = transaction.transaction_info()?;

    let event_type = MoveType::from_str(&format!(
        "{}::fri_statement_contract::FriCtx",
        config.verifier_address
    ))?;
    let fri_ctx_data: InitFriGroup = get_event_from_transaction(&transaction, event_type)?
        .clone()
        .try_into()?;

    let compute_next_layer_event_type = MoveType::from_str(&format!(
        "{}::fri_statement_contract::ComputeNextLayer",
        config.verifier_address
    ))?;
    let compute_next_layer_data: ComputeNextLayer =
        get_event_from_transaction(&transaction, compute_next_layer_event_type)?
            .clone()
            .try_into()?;

    let register_fact_event_type = MoveType::from_str(&format!(
        "{}::fri_statement_contract::RegisterFactVerifyFri",
        config.verifier_address
    ))?;
    let register_fact_data: RegisterFactVerifyFri =
        get_event_from_transaction(&transaction, register_fact_event_type)?
            .clone()
            .try_into()?;

    let stat = StatInfo {
        time: t.elapsed().as_secs_f32(),
        gas_used: transaction_info.gas_used.0,
    };
    Ok((
        fri_ctx_data,
        compute_next_layer_data,
        register_fact_data,
        stat,
    ))
}

pub async fn init_fri_group(config: &AppConfig, data: InitFriGroup) -> anyhow::Result<StatInfo> {
    let t = Instant::now();
    let payload = TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(config.verifier_address, Identifier::new("fri_layer")?),
        Identifier::new("init_fri_group")?,
        vec![],
        serialize_values(&vec![MoveValue::U64(data.fri_ctx)]),
    ));
    let transaction = build_and_submit(
        &config.client,
        payload,
        &config.account,
        config.chain_id,
        Some(5),
        None,
    )
    .await?;
    let transaction_info = transaction.transaction_info()?.clone();
    info!(
        "init_fri_group finished: id={}; hash={}; gas={}",
        transaction_info.version,
        transaction_info.hash.to_string(),
        transaction_info.gas_used
    );
    let stat = StatInfo {
        time: t.elapsed().as_secs_f32(),
        gas_used: transaction_info.gas_used.0,
    };
    Ok(stat)
}

pub async fn compute_next_layer(
    config: &AppConfig,
    data: &ComputeNextLayer,
) -> anyhow::Result<StatInfo> {
    let t = Instant::now();
    let payload = TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(config.verifier_address, Identifier::new("fri_layer")?),
        Identifier::new("compute_next_layer")?,
        vec![],
        serialize_values(&vec![
            MoveValue::U64(data.channel_ptr),
            MoveValue::U64(data.fri_queue_ptr),
            MoveValue::U64(data.merkle_queue_ptr),
            MoveValue::U64(data.n_queries),
            MoveValue::U64(data.fri_ctx),
            MoveValue::U256(data.evaluation_point),
            MoveValue::U64(data.fri_coset_size),
        ]),
    ));
    let transaction = build_and_submit(
        &config.client,
        payload,
        &config.account,
        config.chain_id,
        Some(5),
        None,
    )
    .await?;
    let transaction_info = transaction.transaction_info()?;
    info!(
        "compute_next_layer finished: id={}; hash={}; gas={}",
        transaction_info.version,
        transaction_info.hash.to_string(),
        transaction_info.gas_used
    );
    let stat = StatInfo {
        time: t.elapsed().as_secs_f32(),
        gas_used: transaction_info.gas_used.0,
    };
    Ok(stat)
}

use aptos_sdk::move_types::identifier::Identifier;
use aptos_sdk::move_types::language_storage::ModuleId;
use aptos_sdk::move_types::u256::U256;
use aptos_sdk::move_types::value::{serialize_values, MoveValue};
use aptos_sdk::types::transaction::{EntryFunction, TransactionPayload};
use log::info;

use crate::verify_merkle::{merkle_verifier, VerifyMerkle};

pub async fn register_fact_fri(
    config: &AppConfig,
    data: RegisterFactVerifyFri,
    n_queries: u64,
) -> anyhow::Result<StatInfo> {
    let t = Instant::now();
    let payload = TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(
            config.verifier_address,
            Identifier::new("fri_statement_contract")?,
        ),
        Identifier::new("register_fact_verify_fri")?,
        vec![],
        serialize_values(&vec![
            MoveValue::U64(data.data_to_hash),
            MoveValue::U64(data.fri_queue_ptr),
            MoveValue::U64(n_queries),
        ]),
    ));
    let transaction = build_and_submit(
        &config.client,
        payload,
        &config.account,
        config.chain_id,
        Some(5),
        None,
    )
    .await?;
    let transaction_info = transaction.transaction_info()?;
    info!(
        "register_fact_verify_fri finished: id={}; hash={}; gas={}",
        transaction_info.version,
        transaction_info.hash.to_string(),
        transaction_info.gas_used
    );
    let stat = StatInfo {
        time: t.elapsed().as_secs_f32(),
        gas_used: transaction_info.gas_used.0,
    };
    Ok(stat)
}

pub fn sample_verify_fri_input(
    index: isize,
) -> anyhow::Result<(
    FriVerifyInput,
    MoveValue,
    MoveValue,
    MoveValue,
    MoveValue,
    MoveValue,
)> {
    let file_path = format!("./data/fri_verify/fri_verify_{}.json", index);
    let input_file = File::open(file_path)?;
    let reader = BufReader::new(input_file);
    let fri_verify_input: FriVerifyInput = serde_json::from_reader(reader)?;

    //proof
    let mut proof_vec = vec![];
    for i in 0..fri_verify_input.proof.len() {
        proof_vec.push(MoveValue::U256(U256::from_str(
            &fri_verify_input.proof[i].clone(),
        )?));
    }
    let proof = MoveValue::Vector(proof_vec);

    //queue
    let mut fri_queue_vec = vec![];
    for i in 0..fri_verify_input.fri_queue.len() {
        fri_queue_vec.push(MoveValue::U256(U256::from_str(
            &fri_verify_input.fri_queue[i].clone(),
        )?));
    }
    let fri_queue = MoveValue::Vector(fri_queue_vec);

    let evaluation_point =
        MoveValue::U256(U256::from_str(&fri_verify_input.evaluation_point.clone())?);
    let fri_step_size = MoveValue::U256(U256::from_str(&fri_verify_input.fri_step_size.clone())?);
    let expected_root = MoveValue::U256(U256::from_str(&fri_verify_input.expected_root.clone())?);
    Ok((
        fri_verify_input,
        proof,
        fri_queue,
        evaluation_point,
        fri_step_size,
        expected_root,
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FriVerifyInput {
    pub proof: Vec<String>,
    pub fri_queue: Vec<String>,
    pub evaluation_point: String,
    pub fri_step_size: String,
    pub expected_root: String,
}

#[derive(Clone)]
pub struct VerifyFriTransactionInput {
    pub proof: MoveValue,
    pub fri_queue: MoveValue,
    pub evaluation_point: MoveValue,
    pub fri_step_size: MoveValue,
    pub expected_root: MoveValue,
}

use aptos_sdk::rest_client::aptos_api_types::Event;
use tokio::time::Instant;

#[derive(Debug)]
pub struct InitFriGroup {
    pub fri_ctx: u64,
}

impl TryInto<InitFriGroup> for Event {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<InitFriGroup, Self::Error> {
        Ok(InitFriGroup {
            fri_ctx: str_to_u64(self.data.get("fri_ctx").unwrap().as_str().unwrap())?,
        })
    }
}

#[derive(Debug)]
pub struct ComputeNextLayer {
    pub channel_ptr: u64,
    pub fri_queue_ptr: u64,
    pub merkle_queue_ptr: u64,
    pub n_queries: u64,
    pub fri_ctx: u64,
    pub evaluation_point: U256,
    pub fri_coset_size: u64,
}

impl TryInto<ComputeNextLayer> for Event {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<ComputeNextLayer, Self::Error> {
        Ok(ComputeNextLayer {
            channel_ptr: str_to_u64(self.data.get("channel_ptr").unwrap().as_str().unwrap())?,
            evaluation_point: str_to_u256(
                self.data.get("evaluation_point").unwrap().as_str().unwrap(),
            )?,
            fri_coset_size: str_to_u64(self.data.get("fri_coset_size").unwrap().as_str().unwrap())?,
            fri_ctx: str_to_u64(self.data.get("fri_ctx").unwrap().as_str().unwrap())?,
            fri_queue_ptr: str_to_u64(self.data.get("fri_queue_ptr").unwrap().as_str().unwrap())?,
            merkle_queue_ptr: str_to_u64(
                self.data.get("merkle_queue_ptr").unwrap().as_str().unwrap(),
            )?,
            n_queries: str_to_u64(self.data.get("n_queries").unwrap().as_str().unwrap())?,
        })
    }
}

#[derive(Debug)]
pub struct RegisterFactVerifyFri {
    pub data_to_hash: u64,
    pub fri_queue_ptr: u64,
}

impl TryInto<RegisterFactVerifyFri> for Event {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<RegisterFactVerifyFri, Self::Error> {
        Ok(RegisterFactVerifyFri {
            data_to_hash: str_to_u64(self.data.get("data_to_hash").unwrap().as_str().unwrap())?,
            fri_queue_ptr: str_to_u64(self.data.get("fri_queue_ptr").unwrap().as_str().unwrap())?,
        })
    }
}
