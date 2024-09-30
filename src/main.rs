use serde::Deserialize;
use serde_xml_rs::from_str;
use tokio_postgres::{NoTls, Error};
use std::fs::read_to_string;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct TV {
    #[serde(rename = "channel", default)]
    channels: Vec<Channel>,
}

#[derive(Debug, Deserialize)]
struct Channel {
    #[serde(rename = "display-name")]
    display_name: String,
}

async fn get_channel_ids(display_names: Vec<String>) -> Result<(), Error> {
    // Připojení k Supabase databázi
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

    // Připravte SQL dotaz s operátorem IN
    let query = r#"SELECT id, Channel_name FROM "TV_channels" WHERE Channel_name = ANY($1)"#;
    
    // Proveďte dotaz, kde display_names je seznam jmen
    let rows = client.query(query, &[&display_names]).await?;

    // Uložíme výsledky do HashMap pro snadné vyhledání podle jména kanálu
    let mut channel_map: HashMap<String, i64> = HashMap::new();
    for row in rows {
        let id: i64 = row.get(0);
        let channel_name: String = row.get(1);
        channel_map.insert(channel_name, id);
    }

    // Vypsání ID ve stejném pořadí jako v původním XML
    for display_name in display_names {
        if let Some(id) = channel_map.get(&display_name) {
            println!("ID for {}: {}", display_name, id);
        } else {
            println!("ID for {} not found", display_name);
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Načtení XML souboru guide.xml
    let xml_data = read_to_string("guide.xml")?;

    // Parsování XML do struktury
    let tv: TV = from_str(&xml_data)?;

    // Extrahování jedinečných display-names do Vecu (se zachováním pořadí)
    let mut display_names = Vec::new();
    for channel in tv.channels {
        if !display_names.contains(&channel.display_name) {
            display_names.push(channel.display_name);
        }
    }

    // Získání ID pro každý display-name
    get_channel_ids(display_names).await?;

    Ok(())
}
