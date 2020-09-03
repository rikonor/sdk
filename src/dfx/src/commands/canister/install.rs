use crate::lib::canister_info::CanisterInfo;
use crate::lib::environment::Environment;
use crate::lib::error::{DfxError, DfxResult};
use crate::lib::installers::assets::post_install_store_assets;
use crate::lib::message::UserMessage;
use crate::lib::models::canister_id_store::CanisterIdStore;
use crate::lib::waiter::create_waiter;

use clap::{App, Arg, ArgMatches, SubCommand};
use ic_agent::{Agent, CanisterAttributes, ComputeAllocation, InstallMode, ManagementCanister};
use slog::info;
use std::convert::TryFrom;
use std::str::FromStr;
use tokio::runtime::Runtime;

pub fn construct() -> App<'static, 'static> {
    SubCommand::with_name("install")
        .about(UserMessage::InstallCanister.to_str())
        .arg(
            Arg::with_name("canister_name")
                .takes_value(true)
                .required_unless("all")
                .help(UserMessage::InstallCanisterName.to_str())
                .required(false),
        )
        .arg(
            Arg::with_name("all")
                .long("all")
                .required_unless("canister_name")
                .help(UserMessage::InstallAll.to_str())
                .takes_value(false),
        )
        .arg(
            Arg::with_name("async")
                .help(UserMessage::AsyncResult.to_str())
                .long("async")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("mode")
                .help(UserMessage::InstallMode.to_str())
                .long("mode")
                .short("m")
                .possible_values(&["install", "reinstall", "upgrade"])
                .default_value("install")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("compute-allocation")
                .help(UserMessage::InstallComputeAllocation.to_str())
                .long("compute-allocation")
                .short("c")
                .takes_value(true)
                .validator(compute_allocation_validator),
        )
}

async fn install_canister(
    env: &dyn Environment,
    agent: &Agent,
    canister_info: &CanisterInfo,
    compute_allocation: Option<ComputeAllocation>,
    mode: InstallMode,
) -> DfxResult {
    let mgr = ManagementCanister::new(agent);
    let log = env.get_logger();
    let canister_id = canister_info.get_canister_id().map_err(|_| {
        DfxError::CannotFindBuildOutputForCanister(canister_info.get_name().to_owned())
    })?;

    info!(
        log,
        "Installing code for canister {}, with canister_id {}",
        canister_info.get_name(),
        canister_id.to_text(),
    );

    let wasm_path = canister_info
        .get_output_wasm_path()
        .expect("Cannot get WASM output path.");
    let wasm = std::fs::read(wasm_path)?;

    mgr.install_code(
        create_waiter(),
        &canister_id,
        mode,
        &wasm,
        &[],
        &CanisterAttributes {
            compute_allocation,
            memory_allocation: None,
        },
    )
    .await
    .map_err(DfxError::from)?;

    if canister_info.get_type() == "assets" {
        post_install_store_assets(&canister_info, &agent).await?;
    }

    Ok(())
}

fn compute_allocation_validator(compute_allocation: String) -> Result<(), String> {
    if let Ok(num) = compute_allocation.parse::<u64>() {
        if num <= 100 {
            return Ok(());
        }
    }
    Err("Must be a percent between 0 and 100".to_string())
}

pub fn exec(env: &dyn Environment, args: &ArgMatches<'_>) -> DfxResult {
    let config = env
        .get_config()
        .ok_or(DfxError::CommandMustBeRunInAProject)?;
    let agent = env
        .get_agent()
        .ok_or(DfxError::CommandMustBeRunInAProject)?;
    let compute_allocation = args.value_of("compute-allocation").map(|arg| {
        ComputeAllocation::try_from(arg.parse::<u64>().unwrap())
            .expect("Compute Allocation must be a percentage.")
    });

    let mode = InstallMode::from_str(args.value_of("mode").unwrap())?;

    let mut runtime = Runtime::new().expect("Unable to create a runtime");

    let canister_id_store = CanisterIdStore::for_env(env)?;

    if let Some(canister_name) = args.value_of("canister_name") {
        let canister_id = canister_id_store.get(canister_name)?;

        let canister_info = CanisterInfo::load(&config, canister_name, Some(canister_id))?;
        runtime.block_on(install_canister(
            env,
            &agent,
            &canister_info,
            compute_allocation,
            mode,
        ))?;
        Ok(())
    } else if args.is_present("all") {
        // Install all canisters.
        if let Some(canisters) = &config.get_config().canisters {
            for canister_name in canisters.keys() {
                let canister_id = canister_id_store.get(canister_name)?;

                let canister_info = CanisterInfo::load(&config, canister_name, Some(canister_id))?;
                runtime.block_on(install_canister(
                    env,
                    &agent,
                    &canister_info,
                    compute_allocation,
                    mode.clone(),
                ))?;
            }
        }
        Ok(())
    } else {
        Err(DfxError::CanisterNameMissing())
    }
}
