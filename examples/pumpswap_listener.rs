use chrono::Local;
use hyper_util::rt::TokioIo;
use sol_protos::shredstream::{
    shredstream_proxy_client::ShredstreamProxyClient, SubscribeParsedRequest, TradeType,
};
use tokio::net::UnixStream;
use tonic::transport::Endpoint;
use tower::service_fn;

fn trade_type_str(v: i32) -> &'static str {
    match v {
        x if x == TradeType::PumpSwapBuy as i32 => "BUY_PS",
        x if x == TradeType::PumpSwapSell as i32 => "SELL_PS",
        x if x == TradeType::PumpSwapBuyExactIn as i32 => "BUY_EXACT_PS",
        x if x == TradeType::PumpSwapCreatePool as i32 => "CREATE_POOL_PS",
        x if x == TradeType::PumpSwapCreate as i32 => "CREATE_TOKEN_PS",
        x if x == TradeType::PumpfunBuy as i32 => "BUY_PF",
        x if x == TradeType::PumpfunSell as i32 => "SELL_PF",
        x if x == TradeType::PumpfunBuyExactIn as i32 => "BUY_PF",
        x if x == TradeType::PumpfunCreate as i32 => "CREATE_TOKEN_PF",
        _ => "UNKNOWN",
    }
}

async fn connect(filter: &str) -> Result<tonic::Streaming<sol_protos::shredstream::ParsedTransaction>, Box<dyn std::error::Error>> {
    let channel = Endpoint::try_from("http://[::]:0")?
        .connect_with_connector(service_fn(|_| async {
            let stream = UnixStream::connect("/tmp/grpc.sock").await?;
            Ok::<_, std::io::Error>(TokioIo::new(stream))
        }))
        .await?;
    let mut client = ShredstreamProxyClient::new(channel);
    let stream = client
        .subscribe_parsed_transactions(SubscribeParsedRequest {
            filter: filter.to_string(),
        })
        .await?
        .into_inner();
    Ok(stream)
}

async fn stream_loop(label: &'static str, filter: &'static str) {
    let mut stream = connect(filter).await.expect("connect");
    while let Some(tx) = stream.message().await.unwrap_or(None) {
        let pool = if tx.pool.is_empty() { String::new() } else { format!(" pool={}", tx.pool) };
        println!(
            "{} | {} | {} | slot={} sig={} mint={} signer={} token_amount={} sol_amount={}{}",
            Local::now().format("%Y-%m-%d %H:%M:%S"),
            label,
            trade_type_str(tx.trade_type),
            tx.slot,
            tx.signature,
            tx.mint,
            tx.signer,
            tx.token_amount,
            tx.sol_amount,
            pool,
        );
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Listening for PumpSwap & PumpFun trades + new token/pool creations...");

    let pumpswap = tokio::spawn(stream_loop("PumpSwap", "pumpswap"));
    let pumpfun = tokio::spawn(stream_loop("PumpFun", "pumpfun"));
    let new_tokens = tokio::spawn(stream_loop("PumpFun-New", "pumpfun_new"));

    tokio::try_join!(pumpswap, pumpfun, new_tokens)?;
    Ok(())
}
