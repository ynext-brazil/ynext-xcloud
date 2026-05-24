import requests
import json

# Grab a leaving game ID
url = "https://catalog.gamepass.com/sigls/v2?id=31ff2361-2772-4622-849b-f4f1abb4ad1b&language=pt-BR&market=BR"
res = requests.get(url).json()

if res:
    # Get first 3 leaving games
    ids = [item.get("id") or item.get("Id") for item in res[:3] if isinstance(item, dict) and (item.get("id") or item.get("Id"))]
    ids_query = ",".join(ids)
    
    dc_url = f"https://displaycatalog.mp.microsoft.com/v7.0/products?bigIds={ids_query}&market=BR&languages=pt-BR&MS-CV=DGU1mcuYo0WMMp.0"
    dc_json = requests.get(dc_url).json()
    
    with open("/tmp/leaving.json", "w") as f:
        json.dump(dc_json, f, indent=2)
    print(f"Dumped info for {len(ids)} games.")
