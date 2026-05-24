import requests

url = "https://catalog.gamepass.com/sigls/v2?id=29a81209-df6f-41fd-a528-2ae6b91f719c&language=pt-BR&market=BR"
res = requests.get(url).json()
print("All games len:", len(res))

# What happens if we query without ID?
# print(requests.get("https://catalog.gamepass.com/sigls/v2").text) # Missing param
