use serde::Serialize;

use crate::voicegen_data;

#[derive(Serialize)]
struct Body {
    text: String,
    speaker_id: u32,
}

#[derive(Debug)]
pub enum RequestError {
    Unknown(String),
}

pub async fn request_voice(
    addr: std::net::SocketAddr,
    speaker: u64,
    speech_rate: f64,
    text: &String,
) -> Result<Vec<u8>, RequestError> {
    let client = reqwest::Client::new();

    // Generate query
    let url: String = format!("http://{}/audio_query", addr);
    let audio_query = client
        .post(url)
        .query(&[
            ("text", text.as_str()),
            ("speaker", speaker.to_string().as_str()),
        ])
        .send()
        .await
        .map_err(|e| RequestError::Unknown(e.to_string()))?
        .text()
        .await
        .map_err(|e| RequestError::Unknown(e.to_string()))?;

    let mut audio_query: serde_json::Value =
        serde_json::from_str(audio_query.as_str()).map_err(|e| {
            RequestError::Unknown("failed to deserialize json".to_string() + e.to_string().as_str())
        })?;

    audio_query["speedScale"] = serde_json::json!(speech_rate);

    let audio_query = serde_json::to_string(&audio_query).map_err(|e| {
        RequestError::Unknown("failed to serialize json".to_string() + e.to_string().as_str())
    })?;

    // Generate wav
    let url: String = format!("http://{}/synthesis", addr);
    let data = client
        .post(url)
        .query(&[("speaker", speaker.to_string().as_str())])
        .body(audio_query)
        .send()
        .await
        .map_err(|e| RequestError::Unknown(e.to_string()))?
        .bytes()
        .await
        .map_err(|e| RequestError::Unknown(e.to_string()))?;

    Ok(data.to_vec())
}

pub async fn request_speakers(
    addr: std::net::SocketAddr,
) -> Result<voicegen_data::SpeakersResponse, RequestError> {
    let client = reqwest::Client::new();
    let url: String = format!("http://{}/speakers", addr);

    // Request speakers
    let speakers_response = client
        .get(url)
        .send()
        .await
        .map_err(|e| RequestError::Unknown(e.to_string()))?;

    //println!("{:?}", speakers_response.status());
    let speakers_response = speakers_response.text().await.unwrap();
    //println!("{:?}", speakers_response);

    let speakers_response =
        serde_json::from_str::<voicegen_data::SpeakersResponse>(speakers_response.as_str())
            .map_err(|e| RequestError::Unknown(e.to_string()))?;

    Ok(speakers_response)
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn ts_request_speakers() {
        let speakers_response: voicegen_data::SpeakersResponse =
            request_speakers(std::net::SocketAddr::from(([127, 0, 0, 1], 50031)))
                .await
                .unwrap();

        println!("{:?}", speakers_response);
    }
}
