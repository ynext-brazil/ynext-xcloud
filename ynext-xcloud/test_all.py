import requests

ids = [
    "1bf84c2b-0643-4591-893f-d9edb703f692", # current
    "29a81209-df6f-41fd-a528-2ae6b91f719c"  # old
]
for id in ids:
    url = f"https://catalog.gamepass.com/sigls/v2?id={id}&language=pt-BR&market=BR"
    try:
        res = requests.get(url).json()
        print(f"ID {id} size: {len(res)}")
    except Exception as e:
        print(e)
