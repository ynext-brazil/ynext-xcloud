use std::io::Cursor;

fn main() {
    let url = "https://store-images.s-microsoft.com/image/apps.27624.68326442227858632.21f49c7b-79d7-4647-b847-ecc7a34a7901.1aa31c66-2a52-45d6-8fed-badfb9f25ac6?w=320&h=426&q=80";
    let client = reqwest::blocking::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .build().unwrap();
    let resp = client.get(url).send().unwrap();
    let bytes = resp.bytes().unwrap();
    println!("Bytes: {}", bytes.len());
    
    match image::load_from_memory(&bytes) {
        Ok(_) => println!("Decode SUCCESS"),
        Err(e) => println!("Decode FAILED: {:?}", e),
    }
}
