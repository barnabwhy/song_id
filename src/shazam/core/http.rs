use std::time::SystemTime;
use regex::Regex;
use serde_json::{Value, to_string_pretty};

use crate::shazam::core::thread_messages::*;

use crate::shazam::fingerprinting::signature_format::DecodedSignature;
use crate::shazam::fingerprinting::communication::recognize_song_from_signature;

pub async fn try_recognize_song(signature: DecodedSignature) -> Result<SongRecognizedMessage, String> {
    let timestamp = SystemTime::now();
    
    let json_object = recognize_song_from_signature(&signature).await?;
    
    let mut album_name: Option<String> = None;
    let mut release_year: Option<String> = None;
    
    // Sometimes the idea of trying to write functional poetry hurts
    
    if let Value::Array(sections) = &json_object["track"]["sections"] {
        for section in sections {
            if let Value::String(string) = &section["type"] {
                if string == "SONG" {
                    if let Value::Array(metadata) = &section["metadata"] {
                        for metadatum in metadata {
                            if let Value::String(title) = &metadatum["title"] {
                                if title == "Album" {
                                    if let Value::String(text) = &metadatum["text"] {
                                        album_name = Some(text.to_string());
                                    }
                                }
                                else if title == "Released" {
                                    if let Value::String(text) = &metadatum["text"] {
                                        release_year = Some(text.to_string());
                                    }
                                }
                            }
                        }
                        break;
                    }
                }
            }
        }
    }
    
    Ok(SongRecognizedMessage {
        artist_name: match &json_object["track"]["subtitle"] {
            Value::String(string) => string.to_string(),
            _ => { return Err("No match for this song".to_string()) }
        },
        album_name: album_name,
        song_name: match &json_object["track"]["title"] {
            Value::String(string) => string.to_string(),
            _ => { return Err("No match for this song".to_string()) }
        },
        cover_image: match &json_object["track"]["images"]["coverart"] {
            Value::String(string) => Some(string.to_string()),
            _ => None
        },
        track_seek: match &json_object["matches"][0]["offset"] {
            Value::Number(number) => Some(number.as_f64().unwrap() as f32),
            _ => None
        },
        signature: Box::new(signature),
        track_key: match &json_object["track"]["key"] {
            Value::String(string) => string.to_string(),
            _ => { return Err("No match for this song".to_string()) }
        },
        release_year: release_year,
        genre: match &json_object["track"]["genres"]["primary"] {
            Value::String(string) => Some(string.to_string()),
            _ => None
        },
        shazam_json: Regex::new("\n *").unwrap().replace_all(&
            Regex::new("([,:])\n *").unwrap().replace_all(&
                to_string_pretty(&json_object).unwrap(), "$1 ").into_owned(),
            "").into_owned(),
        timestamp,
    })
}
