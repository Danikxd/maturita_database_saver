use serde::Deserialize;
use serde_xml_rs::from_str;
use tokio_postgres::{NoTls, Error};
use std::fs::read_to_string;
use std::collections::HashMap;

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
    #[serde(rename = "start")]
    start: String,
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

    // Extrahování display names z XML kanálů
    let display_names: Vec<String> = xml_channels.iter().map(|c| c.display_name.clone()).collect();

    // Připravte SQL dotaz s operátorem IN
    let query = r#"SELECT id, Channel_name FROM "TV_channels" WHERE Channel_name = ANY($1)"#;

    // Proveďte dotaz, kde display_names je seznam jmen
    let rows = client.query(query, &[&display_names]).await?;

    // Uložte mapu Channel_name -> ID z dotazu
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

async fn save_programmes(programmes: &[Programme], channel_ids: Vec<i64>) -> Result<(), Error> {
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

    // Start with the first channel ID and track the last processed channel
    let mut current_channel_index = 0;
    let mut last_channel_id = &programmes[0].channel_id;

    // Vytvoření seznamu hodnot pro hromadné vkládání
    let mut values: Vec<(i64, String)> = Vec::new();

    for programme in programmes {
        // Pokud se změnil channel_id, přesuň se k dalšímu kanálu
        if &programme.channel_id != last_channel_id {
            current_channel_index += 1;
            if current_channel_index >= channel_ids.len() {
                break; // Nejsou žádné další kanály v channel_ids
            }
            last_channel_id = &programme.channel_id;
        }

        let channel_id = channel_ids[current_channel_index];

        // Přidání do seznamu hodnot
        values.push((channel_id, programme.title.clone()));
    }

    // Pokud nejsou žádné hodnoty, návrat s prázdným výsledkem
    if values.is_empty() {
        return Ok(());
    }

    // Připravení dotazu pro hromadné vložení
    let mut query = String::from(r#"INSERT INTO "Series" (channel_id, title) VALUES "#);

    // Vytvoření dotazu s více řádky
    let mut params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = Vec::new();
    for (i, (channel_id, title)) in values.iter().enumerate() {
        if i > 0 {
            query.push_str(", ");
        }
        query.push_str(&format!("(${}, ${})", params.len() + 1, params.len() + 2));
        params.push(channel_id);
        params.push(title);
    }

    // Proveďte hromadné vložení
    client.execute(query.as_str(), &params[..]).await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Načtení XML souboru guide.xml
    let xml_data = read_to_string("guide.xml")?;

    // Parsování XML do struktury
    let tv: TV = from_str(&xml_data)?;

    // Získání ID pro každý display-name v pořadí dle XML
    let channel_ids = get_channel_ids(&tv.channels).await?;

    // Uložení programů do databáze
    save_programmes(&tv.programmes, channel_ids).await?;

    Ok(())
}
