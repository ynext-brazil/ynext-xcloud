import requests

ids = [
    "06323672-b8c8-43cc-b0de-32d5a9834749",
    "6a589fa0-d493-472b-8e20-3813699d7056",
    "31ff2361-2772-4622-849b-f4f1abb4ad1b",
    "1bf84c2b-0643-4591-893f-d9edb703f692"
]
for id in ids:
    url = f"https://catalog.gamepass.com/sigls/v2?id={id}&language=pt-BR&market=BR"
    print(id, requests.get(url).status_code)
