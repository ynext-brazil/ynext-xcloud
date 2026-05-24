import requests

urls = [
    "https://catalog.gamepass.com/lists/cloud",
    "https://gamepass.xbox.com/public/surfaces/cloud/lists",
    "https://catalog.gamepass.com/v3/catalog",
    "https://catalog.gamepass.com/catalog/v2",
    "https://api.xbox.com/v2/catalog",
]
for url in urls:
    try:
        r = requests.get(url, timeout=3)
        print(f"{url}: {r.status_code}")
    except Exception as e:
        print(f"{url}: {e}")
