use std::path::PathBuf;

use tokio::sync::mpsc;
use tracing::{error, info};

use crate::errors::RsshError;
use crate::transfer::storage::{TransactionId, UploadRecord, remote_upload_path, safe_remote_name};
use crate::transport::sftp::SftpChannel;

const CHANNEL_BOUND: usize = 16;

#[derive(Debug)]
pub enum UploadRequest {
    /// Upload pre-read bytes (from an authorized handle or clipboard).
    Bytes {
        tx_id: TransactionId,
        data: Vec<u8>,
        original_name: String,
        source_label: PathBuf,
    },
}

#[derive(Debug)]
pub enum UploadResponse {
    Success(UploadRecord),
    Failed { tx_id: TransactionId, error: String },
}

pub struct UploadWorker {
    request_tx: mpsc::Sender<UploadRequest>,
    response_rx: mpsc::Receiver<UploadResponse>,
}

impl UploadWorker {
    pub fn spawn(host: String, socket_path: PathBuf, remote_root: String) -> Self {
        let (request_tx, request_rx) = mpsc::channel::<UploadRequest>(CHANNEL_BOUND);
        let (response_tx, response_rx) = mpsc::channel::<UploadResponse>(CHANNEL_BOUND);

        tokio::spawn(async move {
            worker_loop(host, socket_path, remote_root, request_rx, response_tx).await;
        });

        Self {
            request_tx,
            response_rx,
        }
    }

    pub async fn send(&self, req: UploadRequest) -> Result<(), RsshError> {
        self.request_tx
            .send(req)
            .await
            .map_err(|_| RsshError::Transfer("upload worker shut down".into()))
    }

    pub fn try_recv(&mut self) -> Option<UploadResponse> {
        self.response_rx.try_recv().ok()
    }

    pub async fn recv(&mut self) -> Option<UploadResponse> {
        self.response_rx.recv().await
    }
}

async fn worker_loop(
    host: String,
    socket_path: PathBuf,
    remote_root: String,
    mut request_rx: mpsc::Receiver<UploadRequest>,
    response_tx: mpsc::Sender<UploadResponse>,
) {
    let sftp = match SftpChannel::open(&host, &socket_path).await {
        Ok(s) => s,
        Err(e) => {
            error!("SFTP channel failed to open: {e}");
            while let Some(req) = request_rx.recv().await {
                let tx_id = match &req {
                    UploadRequest::Bytes { tx_id, .. } => *tx_id,
                };
                let _ = response_tx
                    .send(UploadResponse::Failed {
                        tx_id,
                        error: format!("SFTP unavailable: {e}"),
                    })
                    .await;
            }
            return;
        }
    };

    let remote_root_path = PathBuf::from(&remote_root);
    let _ = sftp.ensure_dir(&remote_root_path).await;

    while let Some(req) = request_rx.recv().await {
        let response = process_request(&sftp, &remote_root, req).await;
        if response_tx.send(response).await.is_err() {
            break;
        }
    }

    let _ = sftp.close().await;
}

async fn process_request(
    sftp: &SftpChannel,
    remote_root: &str,
    req: UploadRequest,
) -> UploadResponse {
    let UploadRequest::Bytes {
        tx_id,
        data,
        original_name,
        source_label,
    } = req;

    let remote_name = safe_remote_name(&original_name, tx_id);
    let remote_path = remote_upload_path(remote_root, &remote_name);
    let size = data.len() as u64;

    match sftp.upload_bytes(&data, &remote_path).await {
        Ok(_) => {
            info!(tx = %tx_id, remote = %remote_path.display(), size, "upload committed");
            UploadResponse::Success(UploadRecord {
                tx_id,
                source_label,
                remote_path,
                size,
                committed: true,
            })
        }
        Err(e) => UploadResponse::Failed {
            tx_id,
            error: e.to_string(),
        },
    }
}
