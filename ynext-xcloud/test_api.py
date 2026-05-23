import requests

SIGL_NEW = "f13cf6b4-57e6-4459-89df-dc8b1ce5b9cb"
url = f"https://catalog.gamepass.com/sigls/v2?id={SIGL_NEW}&language=pt-BR&market=BR"
res = requests.get(url).json()

ids = [item.get("id") or item.get("Id") for item in res if (item.get("id") or item.get("Id"))]

print(f"Got {len(ids)} IDs from SIGL_NEW")
if len(ids) > 0:
    ids_query = ",".join(ids[:20])
    dc_url = f"https://displaycatalog.mp.microsoft.com/v7.0/products?bigIds={ids_query}&market=BR&languages=pt-BR&MS-CV=DGU1mcuYo0WMMp.0"
    dc_res = requests.get(dc_url)
    print(f"DisplayCatalog status: {dc_res.status_code}")
    
    dc_json = dc_res.json()
    print(f"Products in DisplayCatalog: {len(dc_json.get('Products', []))}")
