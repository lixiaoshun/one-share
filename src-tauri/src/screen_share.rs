use std::sync::Arc;
use std::pin::Pin;
use std::future::Future;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;
use webrtc::track::track_local::TrackLocal;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScreenShareError {
    #[error("WebRTC error: {0}")]
    WebRTCError(#[from] webrtc::Error),
    #[error("Failed to create peer connection: {0}")]
    PeerConnectionError(String),
}

pub struct ScreenShare {
    peer_connection: Arc<RTCPeerConnection>,
    video_track: Arc<TrackLocalStaticSample>,
}

impl ScreenShare {
    pub async fn new() -> Result<Self, ScreenShareError> {
        let mut media_engine = MediaEngine::default();
        // Configure media codecs
        media_engine.register_default_codecs()?;

        let api = APIBuilder::new()
            .with_media_engine(media_engine)
            .build();

        let config = RTCConfiguration {
            ice_servers: vec![RTCIceServer {
                urls: vec!["stun:stun.l.google.com:19302".to_owned()],
                ..Default::default()
            }],
            ..Default::default()
        };

        let peer_connection = api.new_peer_connection(config).await
            .map_err(|e| ScreenShareError::WebRTCError(e))?;

        let video_track = Arc::new(TrackLocalStaticSample::new(
            RTCRtpCodecCapability {
                mime_type: "video/h264".to_owned(),
                ..Default::default()
            },
            "video".to_owned(),
            "screenshare".to_owned(),
        ));

        Ok(Self {
            peer_connection: Arc::new(peer_connection),
            video_track,
        })
    }

    pub async fn start_sharing(&self) -> Result<(), ScreenShareError> {
        let rtp_sender = self.peer_connection
            .add_track(Arc::clone(&self.video_track) as Arc<dyn TrackLocal + Send + Sync>)
            .await
            .map_err(|e| ScreenShareError::WebRTCError(e))?;

        // Handle ICE candidate gathering
        let _pc = Arc::clone(&self.peer_connection);
        tokio::spawn(async move {
            while let Ok(_candidate) = rtp_sender.read_rtcp().await {
                // Handle RTCP packets
            }
        });

        Ok(())
    }

    pub fn stop_sharing(&self) -> impl Future<Output = Result<(), ScreenShareError>> + Send {
        let peer_connection = Arc::clone(&self.peer_connection);
        async move {
            peer_connection.close().await
                .map_err(|e| ScreenShareError::WebRTCError(e))?;
            Ok(())
        }
    }

    pub fn on_connection_state_change<F>(&self, mut f: F)
    where
        F: FnMut(RTCPeerConnectionState) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync + 'static,
    {
        self.peer_connection.on_peer_connection_state_change(Box::new(move |s| f(s)));
    }
}