import requests
for m in ["BR", "US"]:
    url = f"https://catalog.gamepass.com/sigls/v2?id=f13cf6b4-57e6-4459-89df-dc8b1ce5b9cb&language=en-us&market={m}"
    print(m, requests.get(url).status_code)
