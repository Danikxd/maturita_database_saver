mod model;

use model::series;
use model::tv_channels;

use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, Database, EntityTrait, QueryFilter, Set, TransactionTrait};

use serde::Deserialize;
use serde_xml_rs::from_str;
use std::collections::HashMap;
use std::env;
use std::fs::read_to_string;

use chrono::{DateTime, Utc};
use dotenv::dotenv;

// Use the **async** version of reqwest
use reqwest::get;

#[derive(Debug, Deserialize)]
struct TV {
    #[serde(rename = "channel", default)]
    channels: Vec<Channel>,
    #[serde(rename = "programme", default)]
    programmes: Vec<Programme>,
}

#[derive(Debug, Deserialize)]
struct Channel {
    #[serde(rename = "id")]
    id: String,
    #[serde(rename = "display-name")]
    display_name: String,
}

#[derive(Debug, Deserialize)]
struct Programme {
    #[serde(rename = "start", deserialize_with = "deserialize_datetime")]
    start: DateTime<Utc>,
    #[serde(rename = "stop", deserialize_with = "deserialize_datetime")]
    stop: DateTime<Utc>,
    #[serde(rename = "title")]
    title: String,
    #[serde(rename = "channel")]
    channel_id: String,
    #[serde(rename = "desc")]
    desc: Option<String>,
}

fn deserialize_datetime<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = String::deserialize(deserializer)?;
    // Adjust the format if your actual feed has no space or a different offset format
    let dt = chrono::DateTime::parse_from_str(&s, "%Y%m%d%H%M%S %z")
        .map_err(serde::de::Error::custom)?;
    Ok(dt.with_timezone(&Utc))
}

async fn get_channel_ids(
    db: &impl ConnectionTrait,
    xml_channels: &[Channel],
) -> Result<HashMap<String, i64>, sea_orm::DbErr> {
    let display_names: Vec<String> = xml_channels.iter().map(|c| c.display_name.clone()).collect();

    let channels = tv_channels::Entity::find()
        .filter(tv_channels::Column::ChannelName.is_in(display_names.clone()))
        .all(db)
        .await?;

    let mut channel_name_to_id: HashMap<String, i64> = HashMap::new();
    for channel in channels {
        channel_name_to_id.insert(channel.channel_name.clone(), channel.id);
    }

    let mut xml_channel_to_db_id: HashMap<String, i64> = HashMap::new();
    for xml_channel in xml_channels {
        if let Some(&db_id) = channel_name_to_id.get(&xml_channel.display_name) {
            xml_channel_to_db_id.insert(xml_channel.id.clone(), db_id);
        } else {
            eprintln!(
                "Channel '{}' not found in the database.",
                xml_channel.display_name
            );
        }
    }

    Ok(xml_channel_to_db_id)
}

async fn save_programmes(
    db: &impl ConnectionTrait,
    programmes: &[Programme],
    channel_mapping: &HashMap<String, i64>,
) -> Result<(), sea_orm::DbErr> {
    let mut counter = 0;
    for programme in programmes {
        if let Some(&channel_id) = channel_mapping.get(&programme.channel_id) {
            let new_programme = series::ActiveModel {
                channel_id: Set(channel_id),
                title: Set(programme.title.clone()),
                start: Set(programme.start),
                end: Set(programme.stop),
                desc: Set(programme.desc.clone()),
                ..Default::default()
            };

            counter += 1;
            eprintln!("Inserting programme #{} => {}", counter, programme.title);

            new_programme.insert(db).await?;
        } else {
            eprintln!(
                "Channel ID '{}' not found in channel mapping.",
                programme.channel_id
            );
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();


     let url = env::var("GUIDE_URL")?;
     let response = get(&url).await?;
     let xml_data = response.text().await?;

    

   
    let tv: TV = from_str(&xml_data)?;

    // Connect to the database (async)
    let db_url = env::var("DATABASE_URL")?;
    let db = Database::connect(&db_url).await?;

   
    let txn = db.begin().await?;


    let channel_mapping = get_channel_ids(&txn, &tv.channels).await?;

    // Insert programmes
    save_programmes(&txn, &tv.programmes, &channel_mapping).await?;

    // Commit
    txn.commit().await?;

    Ok(())
}