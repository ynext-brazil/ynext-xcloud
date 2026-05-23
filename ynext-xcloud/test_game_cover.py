import requests
import json

url = "https://catalog.gamepass.com/sigls/v2?id=29a81209-df6f-41fd-a528-2ae6b91f719c&language=pt-BR&market=BR"
res = requests.get(url).json()

ids = [item.get("id") or item.get("Id") for item in res if isinstance(item, dict) and (item.get("id") or item.get("Id"))]
ids_query = ",".join(ids[:5])
dc_url = f"https://displaycatalog.mp.microsoft.com/v7.0/products?bigIds={ids_query}&market=BR&languages=pt-BR&MS-CV=DGU1mcuYo0WMMp.0"
dc_json = requests.get(dc_url).json()

for prod in dc_json.get('Products', []):
    images = prod.get('LocalizedProperties', [{}])[0].get('Images', [])
    print(f"Product: {prod.get('LocalizedProperties', [{}])[0].get('ProductTitle')}")
    for img in images:
        print(f"  {img.get('ImagePurpose')}: {img.get('Uri')}")
