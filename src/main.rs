mod model;

use model::series;
use model::tv_channels;

use sea_orm::TransactionTrait;
use serde::Deserialize;
use serde_xml_rs::from_str;
use std::env;
use std::fs::read_to_string;
use std::collections::HashMap;
use chrono::{DateTime, Utc};





use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, Database,  // DatabaseConnection, DatabaseTransaction,
    EntityTrait, QueryFilter, Set,
};
use tv_channels::Entity as ChannelEntity;
//use series::Entity as SeriesEntity;

use dotenv::dotenv;

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
    desc: Option<String>

}

fn deserialize_datetime<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = String::deserialize(deserializer)?;
    // Parse the string into DateTime<Utc>
    let dt = chrono::DateTime::parse_from_str(&s, "%Y%m%d%H%M%S %z")
        .map_err(serde::de::Error::custom)?;
    Ok(dt.with_timezone(&Utc))
}

async fn get_channel_ids(
    db: &impl ConnectionTrait,
    xml_channels: &[Channel],
) -> Result<HashMap<String, i64>, sea_orm::DbErr> {
    let display_names: Vec<String> = xml_channels.iter().map(|c| c.display_name.clone()).collect();
   

    // Query the database for matching channels
    let channels = ChannelEntity::find()
        .filter(tv_channels::Column::ChannelName.is_in(display_names.clone()))
        .all(db)
        .await?;

    // Map channel names to their IDs
    let mut channel_name_to_id: HashMap<String, i64> = HashMap::new();
    for channel in channels {
        channel_name_to_id.insert(channel.channel_name.clone(), channel.id);
    }

    // Build a mapping from XML channel id to database channel id
    let mut xml_channel_to_db_id: HashMap<String, i64> = HashMap::new();
    for xml_channel in xml_channels {
        if let Some(&id) = channel_name_to_id.get(&xml_channel.display_name) {
            xml_channel_to_db_id.insert(xml_channel.id.clone(), id);
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
    channel_mapping: HashMap<String, i64>,
) -> Result<(), sea_orm::DbErr> {

    let mut counter = 0;
    for programme in programmes {
        if let Some(&channel_id) = channel_mapping.get(&programme.channel_id) {
            // Create an ActiveModel instance
            let new_programme = series::ActiveModel {
                channel_id: Set(channel_id),
                title: Set(programme.title.clone()),
                start: Set(programme.start),
                end: Set(programme.stop),
                desc: Set(programme.desc.clone()), // Keep it as an Option
                ..Default::default()
            };

            // Insert the programme into the Series table
            counter = counter + 1;
            eprintln!("jedem {}", counter);
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

  
    // Read the XML data
    let xml_data = read_to_string("../epg/guide.xml")?;
    let tv: TV = from_str(&xml_data)?;

  
    dotenv().ok();

    
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    
    let db = Database::connect(&db_url).await?;

  
    let txn = db.begin().await?;

    
    let channel_mapping = get_channel_ids(&txn, &tv.channels).await?;

    

  
   //  save_programmes(&txn, &tv.programmes, channel_mapping).await?;

    
   txn.commit().await?;

   

    Ok(())
}
