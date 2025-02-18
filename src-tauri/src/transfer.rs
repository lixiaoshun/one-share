use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::fs::File;
use tokio::sync::mpsc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TransferError {
    #[error("IO error: {0}")]
    IOError(#[from] io::Error),
    #[error("Failed to send progress update: {0}")]
    ProgressError(String),
}

#[derive(Debug, Clone)]
pub struct TransferProgress {
    pub bytes_transferred: u64,
    pub total_bytes: u64,
    pub percentage: f32,
}

pub struct FileTransfer {
    progress_tx: mpsc::Sender<TransferProgress>,
}

impl FileTransfer {
    pub fn new(progress_tx: mpsc::Sender<TransferProgress>) -> Self {
        Self { progress_tx }
    }

    pub async fn send_file<P: AsRef<Path>>(
        &self,
        path: P,
        stream: &mut TcpStream,
    ) -> Result<(), TransferError> {
        let mut file = File::open(path)?;
        let total_size = file.metadata()?.len();
        let mut buffer = vec![0u8; 8192];
        let mut bytes_sent = 0;

        while let Ok(n) = file.read(&mut buffer) {
            if n == 0 { break; }
            stream.write_all(&buffer[..n])?;
            bytes_sent += n as u64;

            let progress = TransferProgress {
                bytes_transferred: bytes_sent,
                total_bytes: total_size,
                percentage: (bytes_sent as f32 / total_size as f32) * 100.0,
            };

            self.progress_tx.send(progress).await
                .map_err(|e| TransferError::ProgressError(e.to_string()))?;
        }

        Ok(())
    }

    pub async fn receive_file<P: AsRef<Path>>(
        &self,
        path: P,
        stream: &mut TcpStream,
        total_size: u64,
    ) -> Result<(), TransferError> {
        let mut file = File::create(path)?;
        let mut buffer = vec![0u8; 8192];
        let mut bytes_received = 0;

        while bytes_received < total_size {
            let n = stream.read(&mut buffer)?;
            if n == 0 { break; }
            file.write_all(&buffer[..n])?;
            bytes_received += n as u64;

            let progress = TransferProgress {
                bytes_transferred: bytes_received,
                total_bytes: total_size,
                percentage: (bytes_received as f32 / total_size as f32) * 100.0,
            };

            self.progress_tx.send(progress).await
                .map_err(|e| TransferError::ProgressError(e.to_string()))?;
        }

        Ok(())
    }

    pub async fn start_server(
        port: u16,
        progress_tx: mpsc::Sender<TransferProgress>,
    ) -> Result<(), TransferError> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port))?;
        
        for stream in listener.incoming() {
            match stream {
                Ok(_stream) => {
                    let _transfer = FileTransfer::new(progress_tx.clone());
                    tokio::spawn(async move {
                        // Handle incoming file transfer
                    });
                }
                Err(e) => {
                    eprintln!("Failed to accept connection: {}", e);
                }
            }
        }
        Ok(())
    }
}