use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    system_transaction,
    transaction::{self, Transaction},
};
use std::{collections::HashMap, error::Error, fs, str::FromStr};
use tokio_stream::StreamExt;
use tonic::transport::channel::ClientTlsConfig;
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::geyser::subscribe_update::UpdateOneof;
use yellowstone_grpc_proto::geyser::{SubscribeRequest, SubscribeRequestFilterBlocksMeta};

#[derive(Serialize, Deserialize, Debug)]
struct YamlFile {
    rpc_url: String,
    geyser_url: String,
    geyser_token: String,
    sender_private_key: Vec<u8>,
    recepient_pyblic_key: String,
}

struct Transfer {
    amount: u64,
    sender_keypair: Keypair,
    recepient_public_key: Pubkey,
}

struct TransferResult {
    from: String,
    to: String,
    signature: Signature,
    status: Option<transaction::Result<()>>,
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

    // Send the transaction
    let signature = client
        .send_and_confirm_transaction(&tx)
        .await
        .expect("Failed to send transaction");

    // Get transaction processing stats
    let tx_status = client
        .get_signature_status(&signature)
        .await
        .expect("Failed to get transaction status");

    Ok(TransferResult {
        from: transfer.sender_keypair.pubkey().to_string(),
        to: transfer.recepient_public_key.to_string(),
        signature: signature,
        status: tx_status,
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config_str = fs::read_to_string("config.yaml").expect("The config YAML file is missing");
    let config: YamlFile =
        serde_yaml::from_str::<YamlFile>(&config_str).expect("Incorrect YAML format");
    let sol_client = RpcClient::new(config.rpc_url);
    let amount: u64 = 1_000_000; // 0.001 SOL in lamports
    let transfer = Transfer {
        amount: amount,
        sender_keypair: Keypair::from_bytes(&config.sender_private_key)?,
        recepient_public_key: Pubkey::from_str(&config.recepient_pyblic_key)?,
    };

    let tls_config = ClientTlsConfig::new().with_native_roots();
    let mut client = GeyserGrpcClient::build_from_shared(config.geyser_url)?
        .x_token(Some(config.geyser_token))?
        .tls_config(tls_config)?
        .connect()
        .await?;

    let mut blocks_meta: HashMap<String, SubscribeRequestFilterBlocksMeta> = HashMap::new();
    blocks_meta.insert("client".to_owned(), SubscribeRequestFilterBlocksMeta {});
    let request: SubscribeRequest = SubscribeRequest {
        slots: HashMap::default(),
        accounts: HashMap::default(),
        transactions: HashMap::default(),
        transactions_status: HashMap::default(),
        entry: HashMap::default(),
        blocks: HashMap::default(),
        blocks_meta: blocks_meta,
        commitment: None,
        accounts_data_slice: Vec::default(),
        ping: None,
        from_slot: None,
    };
    let (_, mut stream) = client.subscribe_with_request(Some(request)).await?;

    // Listen for updates
    while let Some(update) = stream.next().await {
        match update {
            Ok(msg) => {
                if let Some(UpdateOneof::BlockMeta(_)) = msg.update_oneof {
                    println!("New block meta found");
                    let result = make_transfer(&transfer, &sol_client).await?;

                    println!("{} -> {}", result.from, result.to);
                    println!("Signature {}", result.signature);
                    match result.status {
                        Some(status_result) => match status_result {
                            Ok(()) => println!("Transaction status is OK"),
                            Err(e) => println!("Trasaction status got error: {}", e),
                        },
                        None => println!("Transaction has None status."),
                    }
                    println!("--------------------------------------------------------------------------------------\n")
                }
            }
            Err(error) => {
                println!("Error: {error:?}");
                break;
            }
        }
    }

    Ok(())
}
