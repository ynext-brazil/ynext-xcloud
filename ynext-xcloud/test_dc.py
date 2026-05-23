import requests
import json

SIGL_ALL = "29a81209-df6f-41fd-a528-2ae6b91f719c"
url = f"https://catalog.gamepass.com/sigls/v2?id={SIGL_ALL}&language=pt-BR&market=BR"
res = requests.get(url).json()

ids = [item.get("id") or item.get("Id") for item in res if isinstance(item, dict) and (item.get("id") or item.get("Id"))]

if len(ids) > 0:
    ids_query = ",".join(ids[:5])
    dc_url = f"https://displaycatalog.mp.microsoft.com/v7.0/products?bigIds={ids_query}&market=BR&languages=pt-BR&MS-CV=DGU1mcuYo0WMMp.0"
    dc_res = requests.get(dc_url)
    dc_json = dc_res.json()
    
    for prod in dc_json.get('Products', []):
        props = prod.get("MarketProperties", [{}])[0]
        release_date = props.get("OriginalReleaseDate")
        print(f"Game: {prod.get('LocalizedProperties', [{}])[0].get('ProductTitle')}, ReleaseDate: {release_date}")
