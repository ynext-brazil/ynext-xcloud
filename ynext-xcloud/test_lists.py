import requests

url = "https://catalog.gamepass.com/sigls/v2?id=f13cf6b4-57e6-4459-89df-dc8b1ce5b9cb&language=pt-BR&market=BR"
res = requests.get(url)
print("f13cf6b4-57e6-4459-89df-dc8b1ce5b9cb:", res.status_code)

# Try fetching the root catalog?
root = "https://catalog.gamepass.com/v3/catalog" # ??? I don't know the exact endpoint
