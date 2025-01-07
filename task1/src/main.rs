use futures::stream::FuturesUnordered;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use solana_sdk::commitment_config::CommitmentConfig;
use std::fs;
use std::pin::Pin;
use std::str::FromStr;
use std::{error::Error, future::Future};

const LAPORTS_PER_SOL: f64 = 1_000_000_000.;

#[derive(Serialize, Deserialize, Debug)]
struct YamlFile {
    rpc_url: String,
    wallets: Vec<String>,
}

struct WalletBalance {
    address: String,
    balance: u64,
}

fn lamport_to_sol(lamports: u64) -> f64 {
    let sol = lamports as f64 / LAPORTS_PER_SOL;
    return sol;
}

// Function for fetching current balance for a given Solana wallet address
async fn get_balance(
    wallet_address: &str,
    rpc_client: &RpcClient,
) -> Result<WalletBalance, Box<dyn Error>> {
    let commitment_config = CommitmentConfig::processed();
    let balance = rpc_client
        .get_balance_with_commitment(&Pubkey::from_str(wallet_address)?, commitment_config)
        .await?;

    Ok(WalletBalance {
        address: wallet_address.to_string(),
        balance: balance.value,
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = fs::read_to_string("config.yaml").expect("The config YAML file is missing");
    let config_yaml: YamlFile =
        serde_yaml::from_str::<YamlFile>(&config).expect("Incorrect YAML format");
    let rpc_url = config_yaml.rpc_url;
    let rpc_client = RpcClient::new(rpc_url.to_string());
    let wallets: Vec<String> = config_yaml.wallets;

    let mut tasks: FuturesUnordered<_> = FuturesUnordered::<
        Pin<Box<dyn Future<Output = Result<WalletBalance, Box<dyn Error>>>>>,
    >::new();
    for wallet in &wallets {
        tasks.push(Box::pin(get_balance(wallet, &rpc_client)));
    }

    while let Some(result) = tasks.next().await {
        let result = result?;
        println!(
            "wallet: {}, balance {} SOL",
            result.address,
            lamport_to_sol(result.balance)
        );
    }

    Ok(())
}
