import requests
import re

html = requests.get("https://www.xbox.com/pt-BR/play", headers={"User-Agent":"Mozilla/5.0"}).text
guids = set(re.findall(r'[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}', html.lower()))
print(f"Found {len(guids)} GUIDs in HTML.")

for guid in list(guids)[:20]:
    url = f"https://catalog.gamepass.com/sigls/v2?id={guid}&language=pt-BR&market=BR"
    res = requests.get(url)
    if res.status_code == 200:
        try:
            js = res.json()
            if isinstance(js, list) and len(js) > 0 and 'id' in js[0].keys():
                print(f"{guid} is a valid catalog list with {len(js)} items.")
        except:
            pass

