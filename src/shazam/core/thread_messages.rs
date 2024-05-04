use crate::shazam::fingerprinting::signature_format::DecodedSignature;

use std::time::SystemTime;

/// This module contains code used from message-based communication between threads.

pub struct SongRecognizedMessage {
    pub artist_name: String,
    pub album_name: Option<String>,
    pub song_name: String,
    pub cover_image: Option<String>,
    pub track_seek: Option<f32>,
    pub signature: Box<DecodedSignature>,

    // Used only in the CSV export for now:
    pub track_key: String,
    pub release_year: Option<String>,
    pub genre: Option<String>,

    pub shazam_json: String,
    pub timestamp: SystemTime,
}