import requests
import json

url = "https://api.github.com/search/code?q=catalog.gamepass.com/sigls"
headers = {"Accept": "application/vnd.github.v3+json"}
res = requests.get(url, headers=headers)
if res.status_code == 200:
    for item in res.json().get('items', [])[:5]:
        print(item['repository']['full_name'], item['html_url'])
else:
    print("Search failed:", res.status_code, res.text)
