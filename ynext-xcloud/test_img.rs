use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .build()?;
    let url = "https://store-images.s-microsoft.com/image/apps.4357.14251034457610118.9d19a4ff-da6a-4952-b88e-6447c23a7bb7.e5e8a7ea-0701-4ec1-a9f7-64402ebf928e?w=320&h=426&q=80";
    println!("Fetching {}", url);
    let resp = client.get(url).send().await?;
    println!("Status: {}", resp.status());
    let bytes = resp.bytes().await?;
    println!("Bytes: {}", bytes.len());
    Ok(())
}
