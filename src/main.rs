use serde::Deserialize;
use serde_xml_rs::from_str;
use tokio_postgres::{NoTls, Error};
use std::fs::read_to_string;
use std::collections::HashMap;
use chrono::{DateTime, Utc};

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
    #[serde(rename = "title")]
    title: String,
    #[serde(rename = "channel")]
    channel_id: String,
}

async fn get_channel_ids(xml_channels: &[Channel]) -> Result<Vec<i64>, Error> {
    // Připojení k databázi
    let (client, connection) = tokio_postgres::connect(
        "user=postgres.ywpygzgmsrewlcqnhvam password=DanikMarik111 host=aws-0-eu-central-1.pooler.supabase.com port=6543 dbname=postgres",
        NoTls,
    )
    .await?;

    // Spuštění connection handleru v asynchronním režimu
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let display_names: Vec<String> = xml_channels.iter().map(|c| c.display_name.clone()).collect();

 
    let query = r#"SELECT id, Channel_name FROM "TV_channels" WHERE Channel_name = ANY($1)"#;

   
    let rows = client.query(query, &[&display_names]).await?;

   
    let mut channel_name_to_id: HashMap<String, i64> = HashMap::new();
    for row in rows {
        let id: i64 = row.get(0);
        let name: String = row.get(1);
        channel_name_to_id.insert(name, id);
    }

    // Seřazení ID podle pořadí v XML kanálech
    let mut channel_ids: Vec<i64> = Vec::new();
    for channel in xml_channels {
        if let Some(&id) = channel_name_to_id.get(&channel.display_name) {
            channel_ids.push(id);
        } else {
            eprintln!("Channel '{}' not found in the database.", channel.display_name);
        }
    }

    Ok(channel_ids)
}

fn deserialize_datetime<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    let s: String = String::deserialize(deserializer)?;
    // Parse the string into DateTime<Utc>
    let dt = chrono::DateTime::parse_from_str(&s, "%Y%m%d%H%M%S %z")
        .map_err(serde::de::Error::custom)?;

    Ok(dt.with_timezone(&Utc))
}


async fn save_programmes(programmes: &[Programme], channel_ids: Vec<i64>) -> Result<(), Error> {
    // Connect to the database
    let (client, connection) = tokio_postgres::connect(
        "user=postgres.ywpygzgmsrewlcqnhvam password=DanikMarik111 host=aws-0-eu-central-1.pooler.supabase.com port=6543 dbname=postgres",
        NoTls,
    )
    .await?;

    // Spawn the connection handler
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let mut current_channel_index = 0;
    let mut last_channel_id = &programmes[0].channel_id;

    let mut values: Vec<(i64, String, DateTime<Utc>)> = Vec::new();

    for programme in programmes {
     
        if &programme.channel_id != last_channel_id {
            current_channel_index += 1;
            if current_channel_index >= channel_ids.len() {
                break; // No more channels in channel_ids
            }
            last_channel_id = &programme.channel_id;
        }

        let channel_id = channel_ids[current_channel_index];

      
        values.push((channel_id, programme.title.clone(), programme.start));
    }

   
    if values.is_empty() {
        return Ok(());
    }

    
    let mut query = String::from(r#"INSERT INTO "Series" (channel_id, title, time) VALUES "#);

   
    let mut params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = Vec::new();
    for (i, (channel_id, title, start_time)) in values.iter().enumerate() {
        if i > 0 {
            query.push_str(", ");
        }
        query.push_str(&format!(
            "(${}, ${}, ${})",
            params.len() + 1,
            params.len() + 2,
            params.len() + 3
        ));
        params.push(channel_id);
        params.push(title);
        params.push(start_time);
    }

    
    client.execute(query.as_str(), &params[..]).await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    
    let xml_data = read_to_string("guide.xml")?;

   
    let tv: TV = from_str(&xml_data)?;

   
    let channel_ids = get_channel_ids(&tv.channels).await?;

   
    save_programmes(&tv.programmes, channel_ids).await?;

    Ok(())
}



