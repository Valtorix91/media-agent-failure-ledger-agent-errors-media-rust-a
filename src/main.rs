mod infrai_client;
mod media_agent;

use infrai_client::InfraiClient;
use media_agent::{disposition, run, AgentError, MediaJob};

#[tokio::main]
async fn main() {
    if let Err(error) = command().await {
        eprintln!("{error}");
        if let AgentError::Stage { stage, .. } = &error {
            eprintln!("queue decision: {:?}", disposition(*stage));
        }
        std::process::exit(1);
    }
}

async fn command() -> Result<(), AgentError> {
    let mut args = std::env::args().skip(1);
    let job = MediaJob {
        asset_id: args.next().unwrap_or_else(|| "asset-1042".to_owned()),
        job_id: args.next().unwrap_or_else(|| "job-88".to_owned()),
        creator_id: args.next().unwrap_or_else(|| "creator-7".to_owned()),
    };
    let client = InfraiClient::from_env().map_err(|capture| AgentError::Capture {
        original: Box::new(AgentError::Stage {
            stage: media_agent::Stage::Ingest,
            message: "client setup".to_owned(),
        }),
        capture,
    })?;
    let receipt = run(job, &client).await?;
    println!("delivered {} to {}", receipt.asset_id, receipt.creator_id);
    Ok(())
}
