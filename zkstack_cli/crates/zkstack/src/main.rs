use clap::{command, Parser, Subcommand};
use commands::{
    args::{AutocompleteArgs, ContainersArgs, UpdateArgs},
    contract_verifier::ContractVerifierCommands,
    dev::DevCommands,
};
use common::{
    check_general_prerequisites,
    config::{global_config, init_global_config, GlobalConfig},
    error::log_error,
    init_prompt_theme, logger,
    version::version_message,
};
use config::EcosystemConfig;
use xshell::Shell;

use crate::commands::{
    args::ServerArgs, chain::ChainCommands, consensus, ecosystem::EcosystemCommands,
    explorer::ExplorerCommands, external_node::ExternalNodeCommands, prover::ProverCommands,
};

pub mod accept_ownership;
mod commands;
mod consts;
mod defaults;
pub mod external_node;
mod messages;
mod utils;

#[derive(Parser, Debug)]
#[command(
    name = "zkstack",
    version = version_message(env!("CARGO_PKG_VERSION")),
    about
)]
struct ZkStack {
    #[command(subcommand)]
    command: ZkStackSubcommands,
    #[clap(flatten)]
    global: ZkStackGlobalArgs,
}

#[derive(Subcommand, Debug)]
pub enum ZkStackSubcommands {
    /// Create shell autocompletion files
    Autocomplete(AutocompleteArgs),
    /// Ecosystem related commands
    #[command(subcommand, alias = "e")]
    Ecosystem(Box<EcosystemCommands>),
    /// Chain related commands
    #[command(subcommand, alias = "c")]
    Chain(Box<ChainCommands>),
    /// Supervisor related commands
    #[command(subcommand)]
    Dev(DevCommands),
    /// Prover related commands
    #[command(subcommand, alias = "p")]
    Prover(ProverCommands),
    /// Run server
    Server(ServerArgs),
    /// External Node related commands
    #[command(subcommand, alias = "en")]
    ExternalNode(ExternalNodeCommands),
    /// Run containers for local development
    #[command(alias = "up")]
    Containers(ContainersArgs),
    /// Run contract verifier
    #[command(subcommand)]
    ContractVerifier(ContractVerifierCommands),
    /// Run dapp-portal
    Portal,
    /// Run block-explorer
    #[command(subcommand)]
    Explorer(ExplorerCommands),
    /// Consensus utilities
    #[command(subcommand)]
    Consensus(consensus::Command),
    /// Update ZKsync
    #[command(alias = "u")]
    Update(UpdateArgs),
    /// Print markdown help
    #[command(hide = true)]
    Markdown,
}

#[derive(Parser, Debug)]
#[clap(next_help_heading = "Global options")]
struct ZkStackGlobalArgs {
    /// Verbose mode
    #[clap(short, long, global = true)]
    verbose: bool,
    /// Chain to use
    #[clap(long, global = true)]
    chain: Option<String>,
    /// Ignores prerequisites checks
    #[clap(long, global = true)]
    ignore_prerequisites: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    human_panic::setup_panic!();

    // We must parse arguments before printing the intro, because some autogenerated
    // Clap commands (like `--version` would look odd otherwise).
    let zkstack_args = ZkStack::parse();

    match run_subcommand(zkstack_args).await {
        Ok(_) => {}
        Err(error) => {
            log_error(error);
            std::process::exit(1);
        }
    }

    Ok(())
}

async fn run_subcommand(zkstack_args: ZkStack) -> anyhow::Result<()> {
    init_prompt_theme();

    logger::new_empty_line();
    logger::intro();

    let shell = Shell::new().unwrap();

    init_global_config_inner(&shell, &zkstack_args.global)?;

    if !global_config().ignore_prerequisites {
        check_general_prerequisites(&shell);
    }

    match zkstack_args.command {
        ZkStackSubcommands::Autocomplete(args) => commands::autocomplete::run(args)?,
        ZkStackSubcommands::Ecosystem(args) => commands::ecosystem::run(&shell, *args).await?,
        ZkStackSubcommands::Chain(args) => commands::chain::run(&shell, *args).await?,
        ZkStackSubcommands::Dev(args) => commands::dev::run(&shell, args).await?,
        ZkStackSubcommands::Prover(args) => commands::prover::run(&shell, args).await?,
        ZkStackSubcommands::Server(args) => commands::server::run(&shell, args).await?,
        ZkStackSubcommands::Containers(args) => commands::containers::run(&shell, args)?,
        ZkStackSubcommands::ExternalNode(args) => {
            commands::external_node::run(&shell, args).await?
        }
        ZkStackSubcommands::ContractVerifier(args) => {
            commands::contract_verifier::run(&shell, args).await?
        }
        ZkStackSubcommands::Explorer(args) => commands::explorer::run(&shell, args).await?,
        ZkStackSubcommands::Consensus(cmd) => cmd.run(&shell).await?,
        ZkStackSubcommands::Portal => commands::portal::run(&shell).await?,
        ZkStackSubcommands::Update(args) => commands::update::run(&shell, args).await?,
        ZkStackSubcommands::Markdown => {
            clap_markdown::print_help_markdown::<ZkStack>();
        }
    }
    Ok(())
}

fn init_global_config_inner(shell: &Shell, zkstack_args: &ZkStackGlobalArgs) -> anyhow::Result<()> {
    if let Some(name) = &zkstack_args.chain {
        if let Ok(config) = EcosystemConfig::from_file(shell) {
            let chains = config.list_of_chains();
            if !chains.contains(name) {
                anyhow::bail!(
                    "Chain with name {} doesnt exist, please choose one of {:?}",
                    name,
                    &chains
                );
            }
        }
    }
    init_global_config(GlobalConfig {
        verbose: zkstack_args.verbose,
        chain_name: zkstack_args.chain.clone(),
        ignore_prerequisites: zkstack_args.ignore_prerequisites,
    });
    Ok(())
}
