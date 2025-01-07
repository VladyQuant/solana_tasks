use futures::stream::FuturesUnordered;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    system_transaction,
    transaction::{self, Transaction},
};
use std::{
    error::Error,
    fs,
    future::Future,
    pin::Pin,
    str::FromStr,
    time::{Duration, Instant},
};

#[derive(Serialize, Deserialize, Debug)]
struct YamlFile {
    rpc_url: String,
    sender_private_keys: Vec<Vec<u8>>,
    recepient_pyblic_keys: Vec<String>,
}

#[derive(Debug)]
struct Transfer {
    amount: u64,
    sender_keypair: Keypair,
    recepient_public_key: Pubkey,
}

struct TransferResult {
    from: String,
    to: String,
    signature: Signature,
    processing_time: Duration,
    status: Option<transaction::Result<()>>,
}

fn parse_yaml(fpath: &str) -> Result<YamlFile, Box<dyn Error>> {
    let config = fs::read_to_string(fpath).expect("The config YAML file is missing");
    let config_yaml: YamlFile =
        serde_yaml::from_str::<YamlFile>(&config).expect("Incorrect YAML format");
    for (i, item) in config_yaml.sender_private_keys.iter().enumerate() {
        assert_eq!(
            item.len(),
            64,
            "Private key number {} has lenght not equal to 64.",
            i + 1
        );
    }
    assert_eq!(
        config_yaml.sender_private_keys.len(),
        config_yaml.recepient_pyblic_keys.len(),
        "The numbers of sender and recepint wallets is not equal."
    );

    Ok(config_yaml)
}

fn form_transfers(config_yaml: &YamlFile, amount: u64) -> Result<Vec<Transfer>, Box<dyn Error>> {
    let mut transfers: Vec<Transfer> = Vec::new();
    for (send_priv_k, rec_pub_k) in config_yaml
        .sender_private_keys
        .iter()
        .zip(config_yaml.recepient_pyblic_keys.iter())
    {
        let sender_keypair = Keypair::from_bytes(send_priv_k).expect("Invalid sender private key");
        let recepient_public_key =
            Pubkey::from_str(rec_pub_k).expect("Invalid recepient public key");
        transfers.push(Transfer {
            amount,
            sender_keypair,
            recepient_public_key,
        });
    }

    Ok(transfers)
}

async fn make_transfer(
    transfer: &Transfer,
    client: &RpcClient,
) -> Result<TransferResult, Box<dyn Error>> {
    let latest_blockhash = client
        .get_latest_blockhash()
        .await
        .expect("Failed to get latest blockhash");

    let tx: Transaction = system_transaction::transfer(
        &transfer.sender_keypair,
        &transfer.recepient_public_key,
        transfer.amount,
        latest_blockhash,
    );

    // Measure the time before sending the transaction
    let start_time = Instant::now();

    // Send the transaction
    let signature = client
        .send_and_confirm_transaction(&tx)
        .await
        .expect("Failed to send transaction");

    // Measure the time after the transaction is sent
    let end_time = Instant::now();
    let duration = end_time.duration_since(start_time);

    // Get transaction processing stats
    let tx_status = client
        .get_signature_status(&signature)
        .await
        .expect("Failed to get transaction status");

    Ok(TransferResult {
        from: transfer.sender_keypair.pubkey().to_string(),
        to: transfer.recepient_public_key.to_string(),
        signature: signature,
        processing_time: duration,
        status: tx_status,
    })
}

async fn make_transfers(
    transfers: &Vec<Transfer>,
    client: &RpcClient,
) -> Result<(), Box<dyn Error>> {
    let mut tasks: FuturesUnordered<_> = FuturesUnordered::<
        Pin<Box<dyn Future<Output = Result<TransferResult, Box<dyn Error>>>>>,
    >::new();
    for transfer in transfers {
        tasks.push(Box::pin(make_transfer(transfer, &client)));
    }

    while let Some(result) = tasks.next().await {
        let result = result?;

        println!("{} -> {}", result.from, result.to);
        println!("Signature {}", result.signature);
        println!("Processing time {:?}", result.processing_time);
        match result.status {
            Some(status_result) => match status_result {
                Ok(()) => println!("Transaction status is OK"),
                Err(e) => println!("Trasaction status got error: {}", e),
            },
            None => println!("Transaction has None status."),
        }
        println!("--------------------------------------------------------------------------------------\n")
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let amount = 100_000_000; // 0.1 SOL in lamports
    let config_yaml = parse_yaml("config.yaml")?;
    let transfers = form_transfers(&config_yaml, amount)?;
    let client = RpcClient::new_with_commitment(
        config_yaml.rpc_url.to_string(),
        CommitmentConfig::finalized(),
    );

    make_transfers(&transfers, &client).await?;

    Ok(())
}
