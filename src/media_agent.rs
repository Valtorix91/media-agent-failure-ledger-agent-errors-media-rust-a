use crate::infrai_client::{CaptureError, InfraiClient};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Ingest,
    Process,
    Deliver,
}

impl fmt::Display for Stage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Ingest => "ingest",
            Self::Process => "process",
            Self::Deliver => "deliver",
        };
        f.write_str(name)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Disposition {
    StopAsset,
    RetryDelivery,
}

pub fn disposition(stage: Stage) -> Disposition {
    match stage {
        Stage::Ingest | Stage::Process => Disposition::StopAsset,
        Stage::Deliver => Disposition::RetryDelivery,
    }
}

#[derive(Debug)]
pub enum AgentError {
    Stage {
        stage: Stage,
        message: String,
    },
    Capture {
        original: Box<AgentError>,
        capture: CaptureError,
    },
}

impl fmt::Display for AgentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stage { stage, message } => write!(f, "{stage} failed: {message}"),
            Self::Capture { original, capture } => {
                write!(f, "{original}; recording also failed: {capture}")
            }
        }
    }
}

impl std::error::Error for AgentError {}

#[derive(Debug)]
pub struct MediaJob {
    pub asset_id: String,
    pub job_id: String,
    pub creator_id: String,
}

#[derive(Debug)]
pub struct DeliveryReceipt {
    pub asset_id: String,
    pub creator_id: String,
}

pub async fn run(job: MediaJob, client: &InfraiClient) -> Result<DeliveryReceipt, AgentError> {
    run_stage(Stage::Ingest, &job, client).await?;
    run_stage(Stage::Process, &job, client).await?;
    run_stage(Stage::Deliver, &job, client).await?;
    Ok(DeliveryReceipt {
        asset_id: job.asset_id,
        creator_id: job.creator_id,
    })
}

async fn run_stage(stage: Stage, job: &MediaJob, client: &InfraiClient) -> Result<(), AgentError> {
    if std::env::var("FAIL_STAGE").ok().as_deref() != Some(stage.to_string().as_str()) {
        return Ok(());
    }

    let original = AgentError::Stage {
        stage,
        message: format!(
            "asset {} in job {} did not complete",
            job.asset_id, job.job_id
        ),
    };
    let event_id = format!("{}:{}:{}", job.job_id, job.asset_id, stage);
    match client.capture(&original.to_string(), &event_id).await {
        Ok(()) => Err(original),
        Err(capture) => Err(AgentError::Capture {
            original: Box::new(original),
            capture,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delivery_is_retryable_but_asset_pipeline_failures_stop() {
        assert_eq!(disposition(Stage::Ingest), Disposition::StopAsset);
        assert_eq!(disposition(Stage::Process), Disposition::StopAsset);
        assert_eq!(disposition(Stage::Deliver), Disposition::RetryDelivery);
    }
}
