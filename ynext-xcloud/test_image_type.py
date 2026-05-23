import requests

url = "https://store-images.s-microsoft.com/image/apps.27624.68326442227858632.21f49c7b-79d7-4647-b847-ecc7a34a7901.1aa31c66-2a52-45d6-8fed-badfb9f25ac6?w=320&h=426&q=80"
headers = {"User-Agent": "Mozilla/5.0"}
r = requests.get(url, headers=headers)

if r.status_code == 200:
    content_type = r.headers.get("Content-Type")
    print(f"Success! Content-Type: {content_type}, Size: {len(r.content)}")
    with open("test_out.img", "wb") as f:
        f.write(r.content)
else:
    print(f"Failed: {r.status_code}")
