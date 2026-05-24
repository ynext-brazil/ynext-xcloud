import urllib.request
import re

url = "https://www.xbox.com/pt-BR/play"
req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64)'})
try:
    html = urllib.request.urlopen(req).read().decode('utf-8')
    matches = re.findall(r'"id":"([0-9a-f-]{36})","title":"([^"]+)"', html)
    unique = {m[1]: m[0] for m in matches}
    for title, i in unique.items():
        print(f"TITLE: {title} | ID: {i}")
except Exception as e:
    print(e)
