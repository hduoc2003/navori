pub mod config;
pub mod register_continuous_memory_page;
pub mod utils;
pub mod verify_fri;
pub mod verify_merkle;
pub mod verify_proof_and_register;

use crate::config::{AppConfig, Config, GlobalStat, StatInfo};
use crate::register_continuous_memory_page::{
    register_continuous_memory_page, sample_register_continuous_page,
};
use crate::verify_fri::{sample_verify_fri_input, verify_fri};
use crate::verify_merkle::{sample_verify_merkle_input, verify_merkle};
use crate::verify_proof_and_register::{sample_vpar_data, verify_proof_and_register};
use log::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut config = AppConfig::from(Config::from_path("config.toml")?);

    let mut stat = GlobalStat::default();

    let sequence_number = config
        .client
        .get_account(config.account.address())
        .await?
        .into_inner()
        .sequence_number;
    config.account.set_sequence_number(sequence_number);

    for i in 1..=3 {
        let merkle_input = sample_verify_merkle_input(i)?;
        stat.verify_merkle
            .push(verify_merkle(&mut config, merkle_input).await?);
    }

    for i in 1..=8 {
        let (fri_verify_input, proof, fri_queue, evaluation_point, fri_step_size, expected_root) =
            sample_verify_fri_input(i)?;
        stat.verify_fri.push(
            verify_fri(
                &config,
                fri_verify_input,
                proof,
                fri_queue,
                evaluation_point,
                fri_step_size,
                expected_root,
            )
            .await?,
        );
        info!("Verify FRI {} success", i);
    }

    for i in 1..=1 {
        let register_continuous_page_input = sample_register_continuous_page(i)?;
        stat.rcmp
            .push(register_continuous_memory_page(&config, register_continuous_page_input).await?);
        info!("Register continuous page {} success", i);
    }

    stat.vpar = verify_proof_and_register(&config, &sample_vpar_data(1).unwrap()).await?;

    let mut verify_merkle_stat = StatInfo {
        time: 0.0,
        gas_used: 0,
    };
    println!("verify merkle");
    for (i, x) in stat.verify_merkle.iter().enumerate() {
        println!(
            "[{}], [{:.2} giây\\ {} gas], [{:.2} giây\\ {} gas], [{:.2} giây\\ {} gas],",
            i + 1,
            x.msc_verify_merkle.time,
            x.msc_verify_merkle.gas_used,
            x.mv_verify_merkle.time,
            x.mv_verify_merkle.gas_used,
            x.register_fact_verify_merkle.time,
            x.register_fact_verify_merkle.gas_used
        );
        verify_merkle_stat.gas_used += x.msc_verify_merkle.gas_used
            + x.mv_verify_merkle.gas_used
            + x.register_fact_verify_merkle.gas_used;
        verify_merkle_stat.time +=
            x.msc_verify_merkle.time + x.mv_verify_merkle.time + x.register_fact_verify_merkle.time;
    }

    let mut verify_fri_stat = StatInfo {
        time: 0.0,
        gas_used: 0,
    };
    println!("verify fri");
    for (i, x) in stat.verify_fri.iter().enumerate() {
        println!(
            "[{}], [{:.2} giây\\ {} gas], [{:.2} giây\\ {} gas], [{:.2} giây\\ {} gas], [{:.2} giây\\ {} gas], [{:.2} giây\\ {} gas],",
            i+1,
            x.verify_fri.time,
            x.verify_fri.gas_used,
            x.init_fri_group.time,
            x.init_fri_group.gas_used,
            x.compute_next_layer.time,
            x.compute_next_layer.gas_used,
            x.verify_merkle.time,
            x.verify_merkle.gas_used,
            x.register_fact_verify_fri.time,
            x.register_fact_verify_fri.gas_used
        );
        verify_fri_stat.gas_used += x.verify_fri.gas_used;
        verify_fri_stat.gas_used += x.init_fri_group.gas_used;
        verify_fri_stat.gas_used += x.compute_next_layer.gas_used;
        verify_fri_stat.gas_used += x.verify_merkle.gas_used;
        verify_fri_stat.gas_used += x.register_fact_verify_fri.gas_used;

        verify_fri_stat.time += x.verify_fri.time;
        verify_fri_stat.time += x.init_fri_group.time;
        verify_fri_stat.time += x.compute_next_layer.time;
        verify_fri_stat.time += x.verify_merkle.time;
        verify_fri_stat.time += x.register_fact_verify_fri.time;
    }

    let mut rcmp_stat = StatInfo {
        time: 0.0,
        gas_used: 0,
    };

    println!("rcmp");
    for (i, x) in stat.rcmp.iter().enumerate() {
        println!(
            "[{}], [{:.2}], [{}],",
            i + 1,
            x.time,
            x.gas_used,
        );
        rcmp_stat.gas_used += x.gas_used;
        rcmp_stat.time += x.time;
    }

    let mut vpar_stat = StatInfo {
        time: 0.0,
        gas_used: 0,
    };

    println!("vpar");
    println!("[{:.2}], [{}],", stat.vpar.prepush_task_metadata.time, stat.vpar.prepush_task_metadata.gas_used);
    vpar_stat.gas_used += stat.vpar.prepush_task_metadata.gas_used;
    vpar_stat.time += stat.vpar.prepush_task_metadata.time;
    println!("[{:.2}], [{}],", stat.vpar.prepush_data.time, stat.vpar.prepush_data.gas_used);
    vpar_stat.gas_used += stat.vpar.prepush_data.gas_used;
    vpar_stat.time += stat.vpar.prepush_data.time;
    for (i, x) in stat.vpar.vpar.iter().enumerate() {
        println!(
            "[{}], [{:.2}], [{}],",
            i + 1,
            x.time,
            x.gas_used,
        );
        vpar_stat.gas_used += x.gas_used;
        vpar_stat.time += x.time;
    }
    println!("[{:.2}], [{}],", stat.vpar.reset_data.time, stat.vpar.reset_data.gas_used);
    vpar_stat.gas_used += stat.vpar.reset_data.gas_used;
    vpar_stat.time += stat.vpar.reset_data.time;

    dbg!(verify_merkle_stat);
    dbg!(verify_fri_stat);
    dbg!(rcmp_stat);
    dbg!(vpar_stat);
    Ok(())
}
