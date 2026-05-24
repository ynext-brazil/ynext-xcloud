import requests

url = "https://catalog.gamepass.com/sigls/v2?id=f13cf6b4-57e6-4459-89df-6aec18cf0538&language=pt-BR&market=BR"
res = requests.get(url)
print("status:", res.status_code)
if res.status_code == 200:
    print("items:", len(res.json()))
