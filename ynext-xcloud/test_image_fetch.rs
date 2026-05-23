use std::error::Error;
use reqwest::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let url = "https://store-images.s-microsoft.com/image/apps.27624.68326442227858632.21f49c7b-79d7-4647-b847-ecc7a34a7901.1aa31c66-2a52-45d6-8fed-badfb9f25ac6?w=320&h=426&q=80";
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .build()?;
    
    let resp = client.get(url).send().await?;
    println!("Status: {}", resp.status());
    let bytes = resp.bytes().await?;
    println!("Bytes: {}", bytes.len());
    
    match image::load_from_memory(&bytes) {
        Ok(_) => println!("Image load SUCCESS"),
        Err(e) => println!("Image load FAILED: {}", e),
    }
    
    Ok(())
}
